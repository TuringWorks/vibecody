import React, { useState, useRef } from "react";
import { AlertCircle, AlertTriangle, Info, ChevronDown } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { PROVIDER_DEFAULT_MODEL } from "../hooks/useModelRegistry";

interface BugReport {
 id: string;
 severity: "critical" | "high" | "medium" | "low" | "info";
 category: "security" | "bug" | "perf" | "style" | "smell";
 title: string;
 description: string;
 file_path: string | null;
 line_hint: number | null;
 suggestion: string;
 fix_snippet: string | null;
}

interface BugBotPanelProps {
 workspacePath?: string;
 provider?: string;
 onOpenFile?: (path: string, line?: number) => void;
}

const SEVERITY_COLOR: Record<string, string> = {
 critical: "var(--accent-rose)",
 high: "var(--accent-gold)",
 medium: "var(--accent-gold)",
 low: "var(--accent-green)",
 info: "var(--info-color)",
};

const SEVERITY_ICON: Record<string, React.ReactNode> = {
 critical: <AlertCircle size={12} strokeWidth={1.5} style={{ color: "var(--accent-rose)" }} />,
 high: <AlertTriangle size={12} strokeWidth={1.5} style={{ color: "var(--accent-gold)" }} />,
 medium: <AlertTriangle size={12} strokeWidth={1.5} style={{ color: "var(--accent-gold)" }} />,
 low: <Info size={12} strokeWidth={1.5} style={{ color: "var(--accent-green)" }} />,
 info: <Info size={12} strokeWidth={1.5} style={{ color: "var(--accent-blue)" }} />,
};

const CATEGORY_LABEL: Record<string, string> = {
 security: "Security",
 bug: "Bug",
 perf: "Performance",
 style: "Style",
 smell: "Code Smell",
};

export function BugBotPanel({ workspacePath, provider, onOpenFile }: BugBotPanelProps) {
 const [reports, setReports] = useState<BugReport[]>([]);
 const [scanning, setScanning] = useState(false);
 const [scanScope, setScanScope] = useState("workspace");
 const [customFile, setCustomFile] = useState("");
 const [error, setError] = useState<string | null>(null);
 const [expanded, setExpanded] = useState<string | null>(null);
 const [filterSeverity, setFilterSeverity] = useState<string>("all");
 const [filterCategory, setFilterCategory] = useState<string>("all");
 const cancelRef = useRef(false);
 const scanIdRef = useRef(0);

 function handleSuspend() {
 cancelRef.current = true;
 setScanning(false);
 setError("Scan suspended by user.");
 }

 async function runScan() {
 if (!workspacePath) {
 setError("Open a workspace folder first.");
 return;
 }
 cancelRef.current = false;
 const thisId = ++scanIdRef.current;
 setScanning(true);
 setError(null);
 setReports([]);
 try {
 const scope = scanScope === "file" && customFile.trim()
 ? `file:${customFile.trim()}`
 : "workspace";
 const model = provider ? (PROVIDER_DEFAULT_MODEL[provider] || "") : undefined;
 const result = await invoke<BugReport[]>("run_bugbot", {
 workspacePath,
 scanScope: scope,
 provider: provider || null,
 model: model || null,
 });
 if (cancelRef.current || scanIdRef.current !== thisId) return;
 setReports(result);
 } catch (e) {
 if (cancelRef.current || scanIdRef.current !== thisId) return;
 setError(String(e));
 } finally {
 if (scanIdRef.current === thisId) setScanning(false);
 }
 }

 const filtered = reports.filter((r) => {
 if (filterSeverity !== "all" && r.severity !== filterSeverity) return false;
 if (filterCategory !== "all" && r.category !== filterCategory) return false;
 return true;
 });

 const countsBySeverity = Object.fromEntries(
 ["critical", "high", "medium", "low", "info"].map((s) => [s, reports.filter((r) => r.severity === s).length])
 );

 return (
 <div className="panel-container">
 <div className="panel-header">BugBot — AI Code Scanner</div>
 <div className="panel-body" style={{ gap: 10, display: "flex", flexDirection: "column", fontSize: "var(--font-size-md)" }}>
 <p style={{ margin: 0, fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>
 Automatically detects bugs, security vulnerabilities, and code smells using AI.
 </p>

 {/* Scan controls */}
 <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
 <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
 <select
 value={scanScope}
 onChange={(e) => setScanScope(e.target.value)}
 className="panel-select"
 >
 <option value="workspace">Entire Workspace</option>
 <option value="file">Specific File</option>
 </select>
 {scanning ? (
 <button onClick={handleSuspend} className="panel-btn panel-btn-danger" style={{ flexShrink: 0 }}>
 Suspend
 </button>
 ) : (
 <button onClick={runScan} className="panel-btn panel-btn-primary" style={{ flexShrink: 0 }}>
 Run Scan
 </button>
 )}
 </div>

 {scanScope === "file" && (
 <input
 type="text"
 placeholder="Path relative to workspace, e.g. src/main.rs"
 value={customFile}
 onChange={(e) => setCustomFile(e.target.value)}
 className="panel-input panel-input-full"
 style={{ flex: 1 }}
 />
 )}
 </div>

 {error && (
 <div className="panel-error">
 {error}
 </div>
 )}

 {/* Summary badges */}
 {reports.length > 0 && (
 <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
 {(["critical", "high", "medium", "low", "info"] as const).map((s) =>
 countsBySeverity[s] > 0 ? (
 <button
 key={s}
 onClick={() => setFilterSeverity(filterSeverity === s ? "all" : s)}
 style={{
 padding: "2px 8px",
 borderRadius: "var(--radius-xs-plus)",
 border: `1px solid ${SEVERITY_COLOR[s]}`,
 background: filterSeverity === s ? `${SEVERITY_COLOR[s]}33` : "transparent",
 color: SEVERITY_COLOR[s],
 cursor: "pointer",
 fontSize: "var(--font-size-sm)",
 fontWeight: 600,
 }}
 >
 {SEVERITY_ICON[s]} {countsBySeverity[s]} {s}
 </button>
 ) : null
 )}
 <button
 onClick={() => { setFilterSeverity("all"); setFilterCategory("all"); }}
 style={{ padding: "2px 8px", borderRadius: "var(--radius-xs-plus)", border: "1px solid var(--border-color)", background: "transparent", color: "var(--text-secondary)", cursor: "pointer", fontSize: "var(--font-size-sm)" }}
 >
 Clear filters
 </button>
 <span style={{ marginLeft: "auto", fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", alignSelf: "center" }}>
 {filtered.length}/{reports.length} shown
 </span>
 </div>
 )}

 {/* Category filter */}
 {reports.length > 0 && (
 <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
 {(["all", "security", "bug", "perf", "style", "smell"] as const).map((c) => (
 <button
 key={c}
 onClick={() => setFilterCategory(c)}
 style={{
 padding: "2px 8px",
 borderRadius: "var(--radius-xs-plus)",
 border: "1px solid var(--border-color)",
 background: filterCategory === c ? "var(--bg-secondary)" : "transparent",
 color: filterCategory === c ? "var(--text-primary)" : "var(--text-secondary)",
 cursor: "pointer",
 fontSize: "var(--font-size-sm)",
 }}
 >
 {c === "all" ? "All" : CATEGORY_LABEL[c]}
 </button>
 ))}
 </div>
 )}

 {/* Issue list */}
 <div style={{ flex: 1, overflowY: "auto", display: "flex", flexDirection: "column", gap: 6 }}>
 {scanning && (
 <div style={{ textAlign: "center", padding: 24, color: "var(--text-secondary)", fontSize: "var(--font-size-md)" }}>
 Analyzing code with AI…<br />
 <span style={{ fontSize: "var(--font-size-sm)", opacity: 0.7 }}>This may take 15–30 seconds</span>
 </div>
 )}

 {!scanning && reports.length === 0 && !error && (
 <div style={{ textAlign: "center", padding: 24, color: "var(--text-secondary)", fontSize: "var(--font-size-md)", lineHeight: 1.7 }}>
 No scan results yet.<br />
 Click <strong>Run Scan</strong> to analyze your workspace.
 </div>
 )}

 {filtered.map((report) => (
 <div
 key={report.id}
 style={{
 border: `1px solid ${SEVERITY_COLOR[report.severity]}44`,
 borderLeft: `3px solid ${SEVERITY_COLOR[report.severity]}`,
 borderRadius: "var(--radius-sm)",
 background: "var(--bg-tertiary)",
 }}
 >
 {/* Issue header */}
 <div role="button" tabIndex={0}
 onClick={() => setExpanded(expanded === report.id ? null : report.id)}
 style={{ display: "flex", alignItems: "flex-start", gap: 8, padding: "8px 12px", cursor: "pointer" }}
 >
 <span style={{ fontSize: "var(--font-size-lg)", flexShrink: 0, marginTop: 1 }}>{SEVERITY_ICON[report.severity]}</span>
 <div style={{ flex: 1, minWidth: 0 }}>
 <div style={{ fontWeight: 600, fontSize: "var(--font-size-base)" }}>{report.title}</div>
 <div style={{ display: "flex", gap: 6, marginTop: 3, flexWrap: "wrap" }}>
 <span style={{ fontSize: "var(--font-size-xs)", padding: "1px 4px", borderRadius: 3, background: `${SEVERITY_COLOR[report.severity]}22`, color: SEVERITY_COLOR[report.severity], fontWeight: 600 }}>
 {report.severity.toUpperCase()}
 </span>
 <span style={{ fontSize: "var(--font-size-xs)", padding: "1px 4px", borderRadius: 3, background: "var(--bg-secondary)", color: "var(--text-secondary)" }}>
 {CATEGORY_LABEL[report.category] || report.category}
 </span>
 {report.file_path && (
 <span
 onClick={(e) => {
 e.stopPropagation();
 if (onOpenFile) {
 const fullPath = workspacePath && !report.file_path!.startsWith("/")
 ? `${workspacePath}/${report.file_path}`
 : report.file_path!;
 onOpenFile(fullPath, report.line_hint ?? undefined);
 }
 }}
 style={{ fontSize: "var(--font-size-xs)", color: "var(--accent-blue)", fontFamily: "var(--font-mono)", cursor: onOpenFile ? "pointer" : "default", textDecoration: onOpenFile ? "underline" : "none" }}
 title="Open in editor"
 >
 {report.file_path}{report.line_hint ? `:${report.line_hint}` : ""}
 </span>
 )}
 </div>
 </div>
 <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", flexShrink: 0 }}>
 {expanded === report.id ? "" : <ChevronDown size={12} />}
 </span>
 </div>

 {/* Expanded detail */}
 {expanded === report.id && (
 <div style={{ borderTop: "1px solid var(--bg-secondary)", padding: "12px 12px", display: "flex", flexDirection: "column", gap: 8 }}>
 <div>
 <div style={{ fontSize: "var(--font-size-sm)", fontWeight: 600, color: "var(--text-secondary)", marginBottom: 3 }}>PROBLEM</div>
 <div style={{ fontSize: "var(--font-size-base)", lineHeight: 1.6 }}>{report.description}</div>
 </div>
 <div>
 <div style={{ fontSize: "var(--font-size-sm)", fontWeight: 600, color: "var(--text-secondary)", marginBottom: 3 }}>SUGGESTION</div>
 <div style={{ fontSize: "var(--font-size-base)", lineHeight: 1.6, color: "var(--text-success)" }}>{report.suggestion}</div>
 </div>
 {report.fix_snippet && (
 <div>
 <div style={{ fontSize: "var(--font-size-sm)", fontWeight: 600, color: "var(--text-secondary)", marginBottom: 3 }}>FIX</div>
 <pre style={{ fontSize: "var(--font-size-sm)", background: "var(--bg-primary)", padding: 8, borderRadius: "var(--radius-xs-plus)", margin: 0, overflow: "auto", lineHeight: 1.5, color: "var(--text-primary)" }}>
 {report.fix_snippet}
 </pre>
 </div>
 )}
 </div>
 )}
 </div>
 ))}
 </div>

 {reports.length > 0 && (
 <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
 {reports.length} issue{reports.length !== 1 ? "s" : ""} found — click any issue to expand
 </div>
 )}
 </div>
 </div>
 );
}

