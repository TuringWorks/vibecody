/**
 * NotebookPanel — Interactive Code Scratchpad.
 *
 * Executable code cells (bash/python/node/ruby/rust/go) with inline output,
 * markdown cells for notes, and AI-powered cell assistance.
 */
import { useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
// lucide-react icons not needed

interface CellOutput {
 stdout: string;
 stderr: string;
 exit_code: number;
 duration_ms: number;
}

interface Cell {
 id: string;
 type: "code" | "markdown";
 language: string;
 content: string;
 output: CellOutput | null;
 running: boolean;
 editing: boolean;
}

interface NotebookPanelProps {
 workspacePath: string | null;
 provider?: string;
}

const LANGUAGES = ["bash", "python", "javascript", "ruby", "rust", "go"];

const langColor: Record<string, string> = {
 bash: "var(--accent-blue)",
 python: "var(--accent-green)",
 javascript: "var(--accent-gold)",
 ruby: "var(--accent-rose)",
 rust: "var(--accent-gold)",
 go: "#74c7ec",
};

let cellCounter = 0;
function newId(): string {
 return `cell-${++cellCounter}-${Date.now()}`;
}

export function NotebookPanel({ workspacePath, provider }: NotebookPanelProps) {
 const [cells, setCells] = useState<Cell[]>([
 { id: newId(), type: "code", language: "bash", content: "", output: null, running: false, editing: true },
 ]);
 const [runningAll, setRunningAll] = useState(false);
 const textareaRefs = useRef<Record<string, HTMLTextAreaElement | null>>({});

 if (!workspacePath) {
 return (
 <div style={{ padding: 16, opacity: 0.6, textAlign: "center" }}>
 <p>Open a workspace folder to use the notebook.</p>
 </div>
 );
 }

 const updateCell = (id: string, updates: Partial<Cell>) => {
 setCells((prev) => prev.map((c) => (c.id === id ? { ...c, ...updates } : c)));
 };

 const addCell = (type: "code" | "markdown", afterId?: string) => {
 const cell: Cell = {
 id: newId(),
 type,
 language: "bash",
 content: "",
 output: null,
 running: false,
 editing: true,
 };
 setCells((prev) => {
 if (afterId) {
 const idx = prev.findIndex((c) => c.id === afterId);
 const next = [...prev];
 next.splice(idx + 1, 0, cell);
 return next;
 }
 return [...prev, cell];
 });
 };

 const deleteCell = (id: string) => {
 setCells((prev) => prev.filter((c) => c.id !== id));
 };

 const moveCell = (id: string, dir: -1 | 1) => {
 setCells((prev) => {
 const idx = prev.findIndex((c) => c.id === id);
 if (idx < 0) return prev;
 const newIdx = idx + dir;
 if (newIdx < 0 || newIdx >= prev.length) return prev;
 const next = [...prev];
 [next[idx], next[newIdx]] = [next[newIdx], next[idx]];
 return next;
 });
 };

 const runCell = async (id: string) => {
 const cell = cells.find((c) => c.id === id);
 if (!cell || cell.type !== "code" || !cell.content.trim()) return;
 updateCell(id, { running: true, output: null });
 try {
 const out = await invoke<CellOutput>("execute_notebook_cell", {
 workspace: workspacePath,
 language: cell.language,
 code: cell.content,
 });
 updateCell(id, { running: false, output: out });
 } catch (e: unknown) {
 updateCell(id, {
 running: false,
 output: { stdout: "", stderr: String(e), exit_code: -1, duration_ms: 0 },
 });
 }
 };

 const runAll = async () => {
 setRunningAll(true);
 for (const cell of cells) {
 if (cell.type === "code" && cell.content.trim()) {
 await runCell(cell.id);
 }
 }
 setRunningAll(false);
 };

 const clearAll = () => {
 setCells([{ id: newId(), type: "code", language: "bash", content: "", output: null, running: false, editing: true }]);
 };

 const handleAiAssist = async (id: string) => {
 const cell = cells.find((c) => c.id === id);
 if (!cell) return;
 updateCell(id, { running: true });
 try {
 const result = await invoke<string>("ai_notebook_assist", {
 cellCode: cell.content,
 cellOutput: cell.output ? `${cell.output.stdout}\n${cell.output.stderr}` : "",
 question: "",
 provider,
 });
 updateCell(id, {
 running: false,
 output: {
 stdout: result,
 stderr: "",
 exit_code: 0,
 duration_ms: 0,
 },
 });
 } catch (e: unknown) {
 updateCell(id, {
 running: false,
 output: { stdout: "", stderr: `AI error: ${e}`, exit_code: -1, duration_ms: 0 },
 });
 }
 };

 return (
 <div style={{ display: "flex", flexDirection: "column", flex: 1, minHeight: 0, overflow: "hidden" }}>
 {/* Toolbar */}
 <div style={{
 display: "flex", gap: 6, padding: "8px 12px", alignItems: "center",
 borderBottom: "1px solid var(--border-color)", flexWrap: "wrap",
 }}>
 <button onClick={() => addCell("code")} className="panel-btn panel-btn-secondary">+ Code</button>
 <button onClick={() => addCell("markdown")} className="panel-btn panel-btn-secondary">+ Markdown</button>
 <div style={{ flex: 1 }} />
 <button onClick={runAll} disabled={runningAll} className="panel-btn panel-btn-primary">
 {runningAll ? "Running..." : "Run All"}
 </button>
 <button onClick={clearAll} className="panel-btn panel-btn-secondary">Clear All</button>
 <span style={{ fontSize: 11, opacity: 0.5 }}>{cells.length} cell{cells.length !== 1 ? "s" : ""}</span>
 </div>

 {/* Cells */}
 <div style={{ flex: 1, overflowY: "auto", padding: "8px 12px", display: "flex", flexDirection: "column", gap: 8 }}>
 {cells.map((cell, idx) => (
 <div
 key={cell.id}
 style={{
 border: "1px solid var(--border-color)",
 borderRadius: 6,
 background: "var(--bg-secondary)",
 overflow: "hidden",
 }}
 >
 {/* Cell header */}
 <div style={{
 display: "flex", alignItems: "center", gap: 6, padding: "4px 8px",
 borderBottom: "1px solid var(--border-color)",
 fontSize: 11,
 }}>
 <span style={{
 padding: "1px 6px", borderRadius: 4, fontWeight: 600, fontSize: 10,
 background: cell.type === "code" ? (langColor[cell.language] || "var(--text-secondary)") : "var(--accent-purple)",
 color: "var(--bg-tertiary)",
 }}>
 {cell.type === "code" ? cell.language.toUpperCase() : "MD"}
 </span>

 {cell.type === "code" && (
 <select
 value={cell.language}
 onChange={(e) => updateCell(cell.id, { language: e.target.value })}
 style={{
 padding: "1px 4px", fontSize: 10, borderRadius: 3,
 background: "var(--bg-primary)", color: "var(--text-primary)",
 border: "1px solid var(--border-color)",
 }}
 >
 {LANGUAGES.map((l) => <option key={l} value={l}>{l}</option>)}
 </select>
 )}

 <div style={{ flex: 1 }} />

 {cell.type === "code" && (
 <>
 <button
 onClick={() => runCell(cell.id)}
 disabled={cell.running}
 title="Run cell"
 style={{ ...cellBtnStyle, color: "var(--text-success)" }}
 >
 {cell.running ? "..." : ""}
 </button>
 <button
 onClick={() => handleAiAssist(cell.id)}
 disabled={cell.running}
 title="AI Assist"
 style={{ ...cellBtnStyle, color: "var(--text-info)" }}
 >
 AI
 </button>
 </>
 )}
 <button onClick={() => moveCell(cell.id, -1)} disabled={idx === 0} style={cellBtnStyle} title="Move up">↑</button>
 <button onClick={() => moveCell(cell.id, 1)} disabled={idx === cells.length - 1} style={cellBtnStyle} title="Move down">↓</button>
 <button onClick={() => addCell("code", cell.id)} style={cellBtnStyle} title="Insert cell below">+</button>
 <button
 onClick={() => deleteCell(cell.id)}
 disabled={cells.length <= 1}
 style={{ ...cellBtnStyle, color: "var(--text-danger)" }}
 title="Delete cell"
 >
 ✕
 </button>
 </div>

 {/* Cell content */}
 {cell.type === "code" ? (
 <textarea
 ref={(el) => { textareaRefs.current[cell.id] = el; }}
 value={cell.content}
 onChange={(e) => updateCell(cell.id, { content: e.target.value })}
 placeholder="Enter code..."
 onKeyDown={(e) => {
 if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
 e.preventDefault();
 runCell(cell.id);
 }
 }}
 style={{
 width: "100%", minHeight: 60, padding: "8px 10px",
 fontFamily: "var(--font-mono)", fontSize: 12, lineHeight: 1.5,
 background: "var(--bg-primary)", color: "var(--text-primary)",
 border: "none", outline: "none", resize: "vertical",
 boxSizing: "border-box",
 }}
 />
 ) : cell.editing ? (
 <textarea
 value={cell.content}
 onChange={(e) => updateCell(cell.id, { content: e.target.value })}
 onBlur={() => updateCell(cell.id, { editing: false })}
 placeholder="Enter markdown..."
 style={{
 width: "100%", minHeight: 40, padding: "8px 10px",
 fontFamily: "inherit", fontSize: 12, lineHeight: 1.5,
 background: "var(--bg-primary)", color: "var(--text-primary)",
 border: "none", outline: "none", resize: "vertical",
 boxSizing: "border-box",
 }}
 />
 ) : (
 <div
 onClick={() => updateCell(cell.id, { editing: true })}
 style={{
 padding: "8px 10px", fontSize: 12, lineHeight: 1.6,
 cursor: "text", minHeight: 30,
 color: "var(--text-primary)",
 whiteSpace: "pre-wrap",
 }}
 >
 {cell.content || <span style={{ opacity: 0.4 }}>Click to edit markdown...</span>}
 </div>
 )}

 {/* Output */}
 {cell.output && (
 <div style={{
 borderTop: "1px solid var(--border-color)",
 padding: "6px 10px", fontSize: 11, fontFamily: "var(--font-mono)",
 maxHeight: 200, overflowY: "auto",
 background: cell.output.exit_code !== 0 ? "color-mix(in srgb, var(--accent-rose) 5%, transparent)" : "rgba(166,227,161,0.05)",
 }}>
 {/* Status bar */}
 <div style={{ display: "flex", gap: 8, marginBottom: 4, fontSize: 10, opacity: 0.6 }}>
 <span style={{ color: cell.output.exit_code === 0 ? "var(--success-color)" : "var(--error-color)" }}>
 exit: {cell.output.exit_code}
 </span>
 {cell.output.duration_ms > 0 && <span>{cell.output.duration_ms}ms</span>}
 </div>
 {cell.output.stdout && (
 <pre style={{ margin: 0, whiteSpace: "pre-wrap", wordBreak: "break-all", color: "var(--text-primary)" }}>
 {cell.output.stdout}
 </pre>
 )}
 {cell.output.stderr && (
 <pre style={{ margin: 0, whiteSpace: "pre-wrap", wordBreak: "break-all", color: "var(--text-danger)" }}>
 {cell.output.stderr}
 </pre>
 )}
 </div>
 )}
 </div>
 ))}
 </div>
 </div>
 );
}


const cellBtnStyle: React.CSSProperties = {
 background: "none", border: "none", cursor: "pointer",
 fontSize: 12, padding: "0 3px",
 color: "var(--text-primary)", opacity: 0.7,
};
