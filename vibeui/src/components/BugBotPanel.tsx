import React, { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

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
}

const SEVERITY_COLOR: Record<string, string> = {
 critical: "#f38ba8",
 high: "#fab387",
 medium: "#f9e2af",
 low: "#a6e3a1",
 info: "#89b4fa",
};

const SEVERITY_ICON: Record<string, string> = {
 critical: "🔴",
 high: "",
 medium: "🟡",
 low: "🟢",
 info: "",
};

const CATEGORY_LABEL: Record<string, string> = {
 security: "Security",
 bug: "Bug",
 perf: "Performance",
 style: "Style",
 smell: "Code Smell",
};

export function BugBotPanel({ workspacePath }: BugBotPanelProps) {
 const [reports, setReports] = useState<BugReport[]>([]);
 const [scanning, setScanning] = useState(false);
 const [scanScope, setScanScope] = useState("workspace");
 const [customFile, setCustomFile] = useState("");
 const [error, setError] = useState<string | null>(null);
 const [expanded, setExpanded] = useState<string | null>(null);
 const [filterSeverity, setFilterSeverity] = useState<string>("all");
 const [filterCategory, setFilterCategory] = useState<string>("all");

 async function runScan() {
 if (!workspacePath) {
 setError("Open a workspace folder first.");
 return;
 }
 setScanning(true);
 setError(null);
 setReports([]);
 try {
 const scope = scanScope === "file" && customFile.trim()
 ? `file:${customFile.trim()}`
 : "workspace";
 const result = await invoke<BugReport[]>("run_bugbot", {
 workspacePath,
 scanScope: scope,
 });
 setReports(result);
 } catch (e) {
 setError(String(e));
 } finally {
 setScanning(false);
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
 <div style={{ display: "flex", flexDirection: "column", height: "100%", padding: 10, gap: 10, fontSize: 13 }}>
 {/* Header */}
 <div style={{ fontWeight: 600, fontSize: 14 }}>BugBot — AI Code Scanner</div>
 <p style={{ margin: 0, fontSize: 12, color: "var(--text-secondary)" }}>
 Automatically detects bugs, security vulnerabilities, and code smells using AI.
 </p>

 {/* Scan controls */}
 <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
 <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
 <select
 value={scanScope}
 onChange={(e) => setScanScope(e.target.value)}
 style={selectStyle}
 >
 <option value="workspace">Entire Workspace</option>
 <option value="file">Specific File</option>
 </select>
 <button
 onClick={runScan}
 disabled={scanning}
 style={{
 padding: "5px 14px",
 borderRadius: 5,
 border: "none",
 background: scanning ? "#313244" : "var(--accent-blue, #007acc)",
 color: "#fff",
 cursor: scanning ? "default" : "pointer",
 fontWeight: 600,
 fontSize: 12,
 flexShrink: 0,
 }}
 >
 {scanning ? "Scanning…" : "Run Scan"}
 </button>
 </div>

 {scanScope === "file" && (
 <input
 type="text"
 placeholder="Path relative to workspace, e.g. src/main.rs"
 value={customFile}
 onChange={(e) => setCustomFile(e.target.value)}
 style={{ ...selectStyle, flex: 1 }}
 />
 )}
 </div>

 {error && (
 <div style={{ background: "#ff4d4f22", color: "#ff4d4f", borderRadius: 5, padding: "6px 10px", fontSize: 12 }}>
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
 borderRadius: 4,
 border: `1px solid ${SEVERITY_COLOR[s]}`,
 background: filterSeverity === s ? `${SEVERITY_COLOR[s]}33` : "transparent",
 color: SEVERITY_COLOR[s],
 cursor: "pointer",
 fontSize: 11,
 fontWeight: 600,
 }}
 >
 {SEVERITY_ICON[s]} {countsBySeverity[s]} {s}
 </button>
 ) : null
 )}
 <button
 onClick={() => { setFilterSeverity("all"); setFilterCategory("all"); }}
 style={{ padding: "2px 8px", borderRadius: 4, border: "1px solid #45475a", background: "transparent", color: "#6c7086", cursor: "pointer", fontSize: 11 }}
 >
 Clear filters
 </button>
 <span style={{ marginLeft: "auto", fontSize: 11, color: "var(--text-secondary)", alignSelf: "center" }}>
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
 padding: "2px 7px",
 borderRadius: 4,
 border: "1px solid #45475a",
 background: filterCategory === c ? "#313244" : "transparent",
 color: filterCategory === c ? "var(--text-primary)" : "var(--text-secondary)",
 cursor: "pointer",
 fontSize: 11,
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
 <div style={{ textAlign: "center", padding: 24, color: "var(--text-secondary)", fontSize: 13 }}>
 Analyzing code with AI…<br />
 <span style={{ fontSize: 11, opacity: 0.7 }}>This may take 15–30 seconds</span>
 </div>
 )}

 {!scanning && reports.length === 0 && !error && (
 <div style={{ textAlign: "center", padding: 24, color: "var(--text-secondary)", fontSize: 13, lineHeight: 1.7 }}>
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
 borderRadius: 6,
 background: "var(--bg-tertiary)",
 overflow: "hidden",
 }}
 >
 {/* Issue header */}
 <div
 onClick={() => setExpanded(expanded === report.id ? null : report.id)}
 style={{ display: "flex", alignItems: "flex-start", gap: 8, padding: "8px 10px", cursor: "pointer" }}
 >
 <span style={{ fontSize: 14, flexShrink: 0, marginTop: 1 }}>{SEVERITY_ICON[report.severity]}</span>
 <div style={{ flex: 1, minWidth: 0 }}>
 <div style={{ fontWeight: 600, fontSize: 12 }}>{report.title}</div>
 <div style={{ display: "flex", gap: 6, marginTop: 3, flexWrap: "wrap" }}>
 <span style={{ fontSize: 10, padding: "1px 5px", borderRadius: 3, background: `${SEVERITY_COLOR[report.severity]}22`, color: SEVERITY_COLOR[report.severity], fontWeight: 600 }}>
 {report.severity.toUpperCase()}
 </span>
 <span style={{ fontSize: 10, padding: "1px 5px", borderRadius: 3, background: "#ffffff11", color: "var(--text-secondary)" }}>
 {CATEGORY_LABEL[report.category] || report.category}
 </span>
 {report.file_path && (
 <span style={{ fontSize: 10, color: "var(--text-secondary)", fontFamily: "monospace" }}>
 {report.file_path}{report.line_hint ? `:${report.line_hint}` : ""}
 </span>
 )}
 </div>
 </div>
 <span style={{ fontSize: 12, color: "var(--text-secondary)", flexShrink: 0 }}>
 {expanded === report.id ? "" : "▼"}
 </span>
 </div>

 {/* Expanded detail */}
 {expanded === report.id && (
 <div style={{ borderTop: "1px solid #313244", padding: "10px 12px", display: "flex", flexDirection: "column", gap: 8 }}>
 <div>
 <div style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)", marginBottom: 3 }}>PROBLEM</div>
 <div style={{ fontSize: 12, lineHeight: 1.6 }}>{report.description}</div>
 </div>
 <div>
 <div style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)", marginBottom: 3 }}>SUGGESTION</div>
 <div style={{ fontSize: 12, lineHeight: 1.6, color: "#a6e3a1" }}>{report.suggestion}</div>
 </div>
 {report.fix_snippet && (
 <div>
 <div style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)", marginBottom: 3 }}>FIX</div>
 <pre style={{ fontSize: 11, background: "#181825", padding: 8, borderRadius: 4, margin: 0, overflow: "auto", lineHeight: 1.5, color: "#cdd6f4" }}>
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
 <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>
 {reports.length} issue{reports.length !== 1 ? "s" : ""} found — click any issue to expand
 </div>
 )}
 </div>
 );
}

const selectStyle: React.CSSProperties = {
 padding: "4px 8px",
 borderRadius: 4,
 border: "1px solid #45475a",
 background: "#1e1e2e",
 color: "#cdd6f4",
 fontSize: 12,
};
