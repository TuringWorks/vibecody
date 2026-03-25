/**
 * CodeMetricsPanel — Code Metrics & Complexity Analyzer.
 *
 * Scans a workspace for source files, reports language breakdown (LOC,
 * code/comment/blank lines), top-10 largest files, and top-10 most complex
 * files (branch-count proxy for cyclomatic complexity).
 */
import { useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

interface LanguageStat {
 language: string;
 extension: string;
 file_count: number;
 lines: number;
 code_lines: number;
 comment_lines: number;
 blank_lines: number;
}

interface FileComplexity {
 path: string;
 lines: number;
 complexity: number;
 language: string;
}

interface CodeMetrics {
 total_files: number;
 total_lines: number;
 total_code_lines: number;
 total_comment_lines: number;
 total_blank_lines: number;
 languages: LanguageStat[];
 largest_files: FileComplexity[];
 most_complex: FileComplexity[];
}

interface CodeMetricsPanelProps {
 workspacePath: string | null;
}

const LANG_COLORS: Record<string, string> = {
 Rust: "#dea584", TypeScript: "#3178c6", JavaScript: "#f7df1e",
 Python: "#4584b6", Go: "#00add8", "C++": "#f34b7d", C: "#555555",
 Java: "#b07219", "C#": "#178600", Ruby: "#701516", Kotlin: "#a97bff",
 Swift: "#fa7343", Shell: "#89e051", SQL: "#e38c00", HTML: "#e34c26",
 CSS: "#563d7c", JSON: "var(--accent-gold)", YAML: "#cb171e", TOML: "#9c4121",
 Markdown: "#083fa1", Dart: "#00b4ab", Zig: "#ec915c", Lua: "#000080",
};

function pct(part: number, total: number) {
 return total === 0 ? 0 : Math.round((part / total) * 100);
}

function fmt(n: number) {
 return n.toLocaleString();
}

function Bar({ value, max, color }: { value: number; max: number; color: string }) {
 const w = max === 0 ? 0 : Math.max(2, Math.round((value / max) * 100));
 return (
 <div style={{ height: 6, background: "var(--bg-primary)", borderRadius: 3, overflow: "hidden", marginTop: 3 }}>
 <div style={{ width: `${w}%`, height: "100%", background: color, borderRadius: 3, transition: "width 0.3s" }} />
 </div>
 );
}

export function CodeMetricsPanel({ workspacePath }: CodeMetricsPanelProps) {
 const [metrics, setMetrics] = useState<CodeMetrics | null>(null);
 const [scanning, setScanning] = useState(false);
 const [error, setError] = useState<string | null>(null);
 const [view, setView] = useState<"languages" | "files" | "complexity">("languages");
 const cancelRef = useRef(false);
 const taskIdRef = useRef(0);

 const handleSuspend = () => {
 cancelRef.current = true;
 setScanning(false);
 setError("Scan suspended by user.");
 };

 const scan = async () => {
 if (!workspacePath || scanning) return;
 cancelRef.current = false;
 taskIdRef.current += 1;
 const thisId = taskIdRef.current;
 setScanning(true);
 setError(null);
 try {
 const result = await invoke<CodeMetrics>("analyze_code_metrics", { workspace: workspacePath });
 if (cancelRef.current || taskIdRef.current !== thisId) return;
 setMetrics(result);
 } catch (e) {
 if (cancelRef.current || taskIdRef.current !== thisId) return;
 setError(String(e));
 } finally {
 if (!cancelRef.current && taskIdRef.current === thisId) {
 setScanning(false);
 }
 }
 };

 if (!workspacePath) {
 return (
 <div style={{ padding: 24, textAlign: "center", color: "var(--text-muted)", fontSize: 12 }}>
 Open a workspace to analyze code metrics.
 </div>
 );
 }

 const maxLines = metrics ? Math.max(...metrics.languages.map((l) => l.lines), 1) : 1;

 const TAB = (id: typeof view, label: string) => (
 <button
 onClick={() => setView(id)}
 style={{
 padding: "5px 14px", fontSize: 11, fontWeight: view === id ? 600 : 400,
 background: view === id ? "color-mix(in srgb, var(--accent-blue) 15%, transparent)" : "transparent",
 color: view === id ? "var(--accent-blue)" : "var(--text-muted)",
 border: "none", borderBottom: view === id ? "2px solid var(--accent-blue)" : "2px solid transparent",
 cursor: "pointer",
 }}
 >
 {label}
 </button>
 );

 return (
 <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
 {/* Header */}
 <div style={{ padding: "10px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", flexShrink: 0, display: "flex", alignItems: "center", gap: 10 }}>
 <span style={{ fontSize: 16 }}></span>
 <div style={{ flex: 1 }}>
 <div style={{ fontSize: 13, fontWeight: 600 }}>Code Metrics</div>
 {metrics && (
 <div style={{ fontSize: 11, color: "var(--text-muted)" }}>
 {fmt(metrics.total_files)} files · {fmt(metrics.total_lines)} lines · {metrics.languages.length} languages
 </div>
 )}
 </div>
 {scanning ? (
 <button
 onClick={handleSuspend}
 style={{
 padding: "6px 16px", fontSize: 12, fontWeight: 600,
 background: "var(--error-color)", color: "var(--text-primary)",
 border: "none", borderRadius: 4, cursor: "pointer",
 }}
 >
 Suspend
 </button>
 ) : (
 <button
 onClick={scan}
 style={{
 padding: "6px 16px", fontSize: 12, fontWeight: 600,
 background: "var(--accent-blue)", color: "var(--text-primary)",
 border: "none", borderRadius: 4, cursor: "pointer",
 }}
 >
 {metrics ? "↻ Re-scan" : "Scan"}
 </button>
 )}
 </div>

 {error && (
 <div style={{ margin: "8px 12px", padding: "6px 10px", background: "var(--error-bg)", color: "var(--text-danger)", borderRadius: 4, fontSize: 11 }}>
 {error}
 </div>
 )}

 {/* Summary cards */}
 {metrics && (
 <div style={{ display: "flex", gap: 8, padding: "10px 12px", flexShrink: 0, borderBottom: "1px solid var(--border-color)" }}>
 {[
 { label: "Total LOC", value: fmt(metrics.total_lines), sub: "lines" },
 { label: "Code", value: fmt(metrics.total_code_lines), sub: `${pct(metrics.total_code_lines, metrics.total_lines)}%` },
 { label: "Comments", value: fmt(metrics.total_comment_lines), sub: `${pct(metrics.total_comment_lines, metrics.total_lines)}%` },
 { label: "Blank", value: fmt(metrics.total_blank_lines), sub: `${pct(metrics.total_blank_lines, metrics.total_lines)}%` },
 { label: "Files", value: fmt(metrics.total_files), sub: `${metrics.languages.length} langs` },
 ].map(({ label, value, sub }) => (
 <div key={label} style={{ flex: 1, padding: "8px 10px", background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)", textAlign: "center" }}>
 <div style={{ fontSize: 16, fontWeight: 700 }}>{value}</div>
 <div style={{ fontSize: 9, color: "var(--text-muted)", fontWeight: 600, marginTop: 1 }}>{label}</div>
 <div style={{ fontSize: 9, color: "var(--text-muted)", opacity: 0.7 }}>{sub}</div>
 </div>
 ))}
 </div>
 )}

 {/* Sub-tabs */}
 {metrics && (
 <div style={{ display: "flex", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", flexShrink: 0 }}>
 {TAB("languages", `Languages (${metrics.languages.length})`)}
 {TAB("files", `Largest Files`)}
 {TAB("complexity", `Most Complex`)}
 </div>
 )}

 <div style={{ flex: 1, overflow: "auto" }}>
 {!metrics && !scanning && (
 <div style={{ textAlign: "center", padding: "40px 0", color: "var(--text-muted)", fontSize: 12 }}>
 Click Scan to analyse this workspace.
 </div>
 )}

 {/* Languages view */}
 {metrics && view === "languages" && (
 <div style={{ padding: "8px 12px", display: "flex", flexDirection: "column", gap: 6 }}>
 {metrics.languages.map((lang) => {
 const color = LANG_COLORS[lang.language] ?? "var(--accent-blue)";
 return (
 <div key={lang.language} style={{ padding: "10px 12px", background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)" }}>
 <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
 <span style={{ width: 10, height: 10, borderRadius: "50%", background: color, flexShrink: 0, display: "inline-block" }} />
 <span style={{ fontSize: 12, fontWeight: 600, flex: 1 }}>{lang.language}</span>
 <span style={{ fontSize: 10, color: "var(--text-muted)" }}>{lang.file_count} files</span>
 <span style={{ fontSize: 11, fontFamily: "var(--font-mono)", fontWeight: 600 }}>{fmt(lang.lines)} lines</span>
 <span style={{ fontSize: 10, color: "var(--text-muted)", width: 36, textAlign: "right" }}>{pct(lang.lines, metrics.total_lines)}%</span>
 </div>
 <Bar value={lang.lines} max={maxLines} color={color} />
 <div style={{ display: "flex", gap: 14, marginTop: 5, fontSize: 10, color: "var(--text-muted)" }}>
 <span>Code: {fmt(lang.code_lines)} ({pct(lang.code_lines, lang.lines)}%)</span>
 <span>Comments: {fmt(lang.comment_lines)} ({pct(lang.comment_lines, lang.lines)}%)</span>
 <span>Blank: {fmt(lang.blank_lines)}</span>
 </div>
 </div>
 );
 })}
 </div>
 )}

 {/* Largest files view */}
 {metrics && view === "files" && (
 <div style={{ padding: "8px 12px", display: "flex", flexDirection: "column", gap: 3 }}>
 <div style={{ display: "grid", gridTemplateColumns: "1fr 70px 60px", gap: 8, padding: "4px 8px", fontSize: 10, fontWeight: 600, color: "var(--text-muted)", borderBottom: "1px solid var(--border-color)" }}>
 <span>File</span><span style={{ textAlign: "right" }}>Lines</span><span style={{ textAlign: "right" }}>Lang</span>
 </div>
 {metrics.largest_files.map((f, i) => (
 <div key={f.path} style={{ display: "grid", gridTemplateColumns: "1fr 70px 60px", gap: 8, padding: "5px 8px", fontSize: 11, borderBottom: "1px solid var(--border-color)", alignItems: "center" }}>
 <span style={{ fontFamily: "var(--font-mono)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={f.path}>
 <span style={{ color: "var(--text-muted)", marginRight: 6 }}>{i + 1}.</span>{f.path}
 </span>
 <span style={{ textAlign: "right", fontFamily: "var(--font-mono)" }}>{fmt(f.lines)}</span>
 <span style={{ textAlign: "right", fontSize: 10, color: LANG_COLORS[f.language] ?? "var(--accent-blue)" }}>{f.language}</span>
 </div>
 ))}
 </div>
 )}

 {/* Most complex view */}
 {metrics && view === "complexity" && (
 <div style={{ padding: "8px 12px", display: "flex", flexDirection: "column", gap: 3 }}>
 <div style={{ fontSize: 10, color: "var(--text-muted)", padding: "4px 8px 8px", fontStyle: "italic" }}>
 Complexity = count of branch-inducing keywords (if/for/while/match/&&/||…)
 </div>
 <div style={{ display: "grid", gridTemplateColumns: "1fr 80px 80px", gap: 8, padding: "4px 8px", fontSize: 10, fontWeight: 600, color: "var(--text-muted)", borderBottom: "1px solid var(--border-color)" }}>
 <span>File</span><span style={{ textAlign: "right" }}>Complexity</span><span style={{ textAlign: "right" }}>Lines</span>
 </div>
 {metrics.most_complex.map((f, i) => {
 const maxC = metrics.most_complex[0]?.complexity ?? 1;
 const bar = Math.max(4, Math.round((f.complexity / maxC) * 100));
 const color = f.complexity > maxC * 0.7 ? "var(--error-color)" : f.complexity > maxC * 0.4 ? "var(--warning-color)" : "var(--success-color)";
 return (
 <div key={f.path} style={{ padding: "5px 8px", borderBottom: "1px solid var(--border-color)" }}>
 <div style={{ display: "grid", gridTemplateColumns: "1fr 80px 80px", gap: 8, fontSize: 11, alignItems: "center", marginBottom: 3 }}>
 <span style={{ fontFamily: "var(--font-mono)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={f.path}>
 <span style={{ color: "var(--text-muted)", marginRight: 6 }}>{i + 1}.</span>{f.path}
 </span>
 <span style={{ textAlign: "right", fontFamily: "var(--font-mono)", fontWeight: 600, color }}>{fmt(f.complexity)}</span>
 <span style={{ textAlign: "right", fontFamily: "var(--font-mono)", fontSize: 10, color: "var(--text-muted)" }}>{fmt(f.lines)}</span>
 </div>
 <div style={{ height: 4, background: "var(--bg-primary)", borderRadius: 2, overflow: "hidden" }}>
 <div style={{ width: `${bar}%`, height: "100%", background: color, borderRadius: 2 }} />
 </div>
 </div>
 );
 })}
 </div>
 )}
 </div>
 </div>
 );
}
