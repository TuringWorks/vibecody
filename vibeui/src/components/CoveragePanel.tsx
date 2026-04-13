import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { CircleCheck, FlaskConical, Loader2, Play, ChevronDown, ChevronRight } from "lucide-react";
import { usePersistentState } from "../hooks/usePersistentState";

interface FileCoverage {
 path: string;
 covered: number;
 total: number;
 pct: number;
 uncovered_lines: number[];
}

interface CoverageResult {
 framework: string;
 total_pct: number;
 files: FileCoverage[];
 raw_output: string;
}

interface CoveragePanelProps {
 workspacePath: string | null;
}

type Filter = "all" | "partial" | "uncovered";

const pctColor = (pct: number) => {
 if (pct >= 80) return "var(--success-color)";
 if (pct >= 50) return "var(--warning-color)";
 return "var(--error-color)";
};

// Strip ANSI escape codes and handle \r (carriage return) terminal overwrite semantics.
// cargo uses \r to update progress lines in place; we keep only the final visible state.
const processLogLine = (raw: string): string => {
  // eslint-disable-next-line no-control-regex
  const s = raw.replace(/\x1B\[[0-9;]*[mGKHFJSTDCBAHfhu]/g, "");
  const parts = s.split("\r");
  for (let i = parts.length - 1; i >= 0; i--) {
    if (parts[i].trim()) return parts[i];
  }
  return "";
};

const toolLabel: Record<string, string> = {
 "cargo-llvm-cov": "Cargo llvm-cov",
 nyc: "nyc (Istanbul)",
 "npm-coverage": "npm coverage",
 "coverage.py": "coverage.py",
 "go-cover": "Go cover",
};

export function CoveragePanel({ workspacePath }: CoveragePanelProps) {
 const [tool, setTool] = usePersistentState<string | null>("coverage.tool", null);
 const [result, setResult] = usePersistentState<CoverageResult | null>("coverage.result", null);
 const [running, setRunning] = useState(false);
 const [error, setError] = usePersistentState<string | null>("coverage.error", null);
 const [filter, setFilter] = usePersistentState<Filter>("coverage.filter", "all");
 const [expanded, setExpanded] = useState<Set<string>>(new Set());
 const [showRaw, setShowRaw] = usePersistentState("coverage.showRaw", false);
 const [logs, setLogs] = useState<string[]>([]);
 const cancelRef = useRef(false);
 const taskIdRef = useRef(0);
 const unlistenRef = useRef<UnlistenFn | null>(null);
 const logEndRef = useRef<HTMLDivElement>(null);

 useEffect(() => {
 if (!workspacePath) return;
 invoke<string>("detect_coverage_tool", { workspace: workspacePath })
 .then(setTool)
 .catch(() => setTool(null));
 }, [workspacePath]);

 // Auto-scroll log to bottom when new lines arrive
 useEffect(() => {
   logEndRef.current?.scrollIntoView({ behavior: "auto" });
 }, [logs]);

 // Clean up listener on unmount
 useEffect(() => () => { unlistenRef.current?.(); }, []);

 const handleSuspend = () => {
 cancelRef.current = true;
 setRunning(false);
 setError("Suspended by user.");
 unlistenRef.current?.();
 unlistenRef.current = null;
 };

 const handleRun = async () => {
 if (!workspacePath || !tool) return;
 cancelRef.current = false;
 taskIdRef.current += 1;
 const thisId = taskIdRef.current;
 setRunning(true);
 setError(null);
 setResult(null);
 setLogs([]);

 // Subscribe to streaming log lines before invoking
 unlistenRef.current?.();
 unlistenRef.current = await listen<string>("coverage:log", (e) => {
   const line = processLogLine(e.payload);
   if (line !== "") setLogs(prev => [...prev, line]);
 });

 try {
 const r = await invoke<CoverageResult>("run_coverage", {
 workspace: workspacePath,
 tool,
 });
 if (cancelRef.current || taskIdRef.current !== thisId) return;
 // Sort files by pct ascending (worst coverage first)
 r.files.sort((a, b) => a.pct - b.pct);
 setResult(r);
 } catch (e: unknown) {
 if (cancelRef.current || taskIdRef.current !== thisId) return;
 setError(String(e));
 } finally {
 if (taskIdRef.current === thisId) {
   setRunning(false);
   if (unlistenRef.current) {
     unlistenRef.current();
     unlistenRef.current = null;
   }
 }
 }
 };

 const toggleExpand = (path: string) => {
 setExpanded(prev => {
 const next = new Set(prev);
 if (next.has(path)) next.delete(path); else next.add(path);
 return next;
 });
 };

 const filteredFiles = result?.files.filter(f => {
 if (filter === "partial") return f.pct > 0 && f.pct < 100;
 if (filter === "uncovered") return f.pct === 0;
 return true;
 }) ?? [];

 const barWidth = (pct: number) => `${Math.max(2, Math.min(100, pct))}%`;

 return (
 <div style={{ padding: "12px", fontFamily: "var(--font-family)", fontSize: "13px", height: "100%", overflowY: "auto" }}>
 {/* Header */}
 <div style={{ display: "flex", alignItems: "center", gap: "10px", marginBottom: "12px", flexWrap: "wrap" }}>
 <span style={{ fontWeight: "bold", display: "flex", alignItems: "center", gap: 6 }}><FlaskConical size={16} strokeWidth={1.5} />Coverage</span>
 {tool && (
 <span style={{ background: "var(--bg-secondary)", padding: "2px 8px", borderRadius: "4px", fontSize: "11px" }}>
 {toolLabel[tool] ?? tool}
 </span>
 )}
 {!tool && !workspacePath && (
 <span style={{ color: "var(--text-secondary)" }}>No workspace open</span>
 )}
 {!tool && workspacePath && (
 <span style={{ color: "var(--text-secondary)" }}>No coverage tool detected</span>
 )}
 {running ? (
 <button
 onClick={handleSuspend}
 style={{
 marginLeft: "auto",
 background: "var(--error-color)",
 color: "var(--btn-primary-fg)", border: "none", borderRadius: "4px",
 padding: "4px 12px", cursor: "pointer",
 }}
 >
 <Loader2 size={14} strokeWidth={1.5} style={{ display: "inline" }} />Suspend
 </button>
 ) : (
 <button
 onClick={handleRun}
 disabled={!tool || !workspacePath}
 style={{
 marginLeft: "auto",
 background: "var(--accent-blue)", color: "var(--btn-primary-fg)",
 border: "none", borderRadius: "4px",
 padding: "4px 12px", cursor: !tool || !workspacePath ? "default" : "pointer",
 }}
 >
 <Play size={14} strokeWidth={1.5} style={{ display: "inline" }} />Run Coverage
 </button>
 )}
 </div>

 {error && (
 <div style={{ background: "color-mix(in srgb, var(--accent-rose) 10%, transparent)", color: "var(--error-color)", padding: "8px", borderRadius: "4px", marginBottom: "12px", whiteSpace: "pre-wrap" }}>
 {error}
 </div>
 )}

 {/* Live log — shown while running; after completion visible via Raw button */}
 {(running || (!result && logs.length > 0)) && (
   <div style={{ background: "var(--bg-secondary)", borderRadius: "4px", padding: "10px", fontFamily: "var(--font-mono, monospace)", fontSize: "11px", lineHeight: 1.5, overflowY: "auto", overflowX: "auto", maxHeight: "calc(100vh - 160px)", color: "var(--text-secondary)" }}>
     {logs.map((line, i) => (
       <div key={i} style={{ whiteSpace: "pre", minWidth: "max-content" }}>{line || "\u00A0"}</div>
     ))}
     <div ref={logEndRef} />
   </div>
 )}

 {result && (
 <>
 {/* Summary bar */}
 <div style={{ marginBottom: "14px" }}>
 <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "4px" }}>
 <span style={{ color: pctColor(result.total_pct), fontWeight: "bold", fontSize: "16px" }}>
 {result.total_pct.toFixed(1)}%
 </span>
 <span style={{ color: "var(--text-secondary)", fontSize: "11px" }}>
 {result.files.length} files
 </span>
 </div>
 <div style={{ background: "var(--bg-secondary)", borderRadius: "3px", height: "6px", overflow: "hidden" }}>
 <div style={{ background: pctColor(result.total_pct), width: barWidth(result.total_pct), height: "100%", transition: "width 0.4s" }} />
 </div>
 </div>

 {/* Filter tabs */}
 <div style={{ display: "flex", gap: "6px", marginBottom: "10px" }}>
 {(["all", "partial", "uncovered"] as Filter[]).map(f => (
 <button
 key={f}
 onClick={() => setFilter(f)}
 style={{
 background: filter === f ? "var(--accent-blue)" : "var(--bg-secondary)",
 color: filter === f ? "var(--text-primary)" : "var(--text-secondary)",
 border: "none", borderRadius: "4px", padding: "2px 10px",
 cursor: "pointer", fontSize: "11px",
 }}
 >
 {f === "all" ? `All (${result.files.length})` : f === "partial" ? `Partial (${result.files.filter(x => x.pct > 0 && x.pct < 100).length})` : `Uncovered (${result.files.filter(x => x.pct === 0).length})`}
 </button>
 ))}
 <button
 onClick={() => setShowRaw(r => !r)}
 style={{
 marginLeft: "auto",
 background: showRaw ? "var(--accent-blue)" : "var(--bg-secondary)",
 color: showRaw ? "var(--text-primary)" : "var(--text-secondary)",
 border: "none", borderRadius: "4px", padding: "2px 10px",
 cursor: "pointer", fontSize: "11px",
 }}
 >
 Raw
 </button>
 </div>

 {showRaw ? (
 <pre style={{ background: "var(--bg-secondary)", padding: "10px", borderRadius: "4px", fontSize: "11px", overflow: "auto", maxHeight: "400px", whiteSpace: "pre-wrap", fontFamily: "var(--font-mono, monospace)" }}>
 {logs.length > 0 ? logs.join("\n") : result.raw_output || "(no output)"}
 </pre>
 ) : (
 <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
 {filteredFiles.length === 0 && (
 <div style={{ color: "var(--text-secondary)", textAlign: "center", padding: "20px" }}>
 No files match the filter.
 </div>
 )}
 {filteredFiles.map(file => {
 const isExpanded = expanded.has(file.path);
 const shortPath = file.path.split("/").slice(-3).join("/");
 return (
 <div key={file.path} style={{ background: "var(--bg-secondary)", borderRadius: "4px", overflow: "hidden" }}>
 <div
 onClick={() => toggleExpand(file.path)}
 style={{ padding: "6px 10px", cursor: "pointer", display: "flex", alignItems: "center", gap: "8px" }}
 >
 <span style={{ color: "var(--text-secondary)", fontSize: "10px" }}>{isExpanded ? <ChevronDown size={10} /> : <ChevronRight size={10} />}</span>
 <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", color: "var(--text-secondary)" }} title={file.path}>
 {shortPath}
 </span>
 <span style={{ color: pctColor(file.pct), fontWeight: "bold", minWidth: "48px", textAlign: "right" }}>
 {file.pct.toFixed(0)}%
 </span>
 <span style={{ color: "var(--text-secondary)", fontSize: "11px", minWidth: "80px", textAlign: "right" }}>
 {file.covered}/{file.total} lines
 </span>
 <div style={{ width: "80px", background: "var(--bg-primary)", borderRadius: "2px", height: "4px", overflow: "hidden" }}>
 <div style={{ background: pctColor(file.pct), width: barWidth(file.pct), height: "100%" }} />
 </div>
 </div>
 {isExpanded && file.uncovered_lines.length > 0 && (
 <div style={{ padding: "6px 10px 8px 28px", borderTop: "1px solid var(--bg-primary)" }}>
 <span style={{ color: "var(--error-color)", fontSize: "11px" }}>
 Uncovered lines: {file.uncovered_lines.slice(0, 30).join(", ")}
 {file.uncovered_lines.length > 30 && ` … +${file.uncovered_lines.length - 30} more`}
 </span>
 </div>
 )}
 {isExpanded && file.uncovered_lines.length === 0 && (
 <div style={{ padding: "6px 10px 8px 28px", borderTop: "1px solid var(--bg-primary)", color: "var(--success-color)", fontSize: "11px" }}>
 <CircleCheck size={14} strokeWidth={1.5} style={{ display: "inline", verticalAlign: "middle", marginRight: 4 }} />All lines covered
 </div>
 )}
 </div>
 );
 })}
 </div>
 )}
 </>
 )}
 </div>
 );
}
