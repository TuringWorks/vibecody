import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

interface AutofixResult {
 framework: string;
 files_changed: number;
 diff: string;
 stdout: string;
}

const FRAMEWORKS = [
 { value: "clippy", label: "Cargo Clippy (Rust)" },
 { value: "eslint", label: "ESLint (JS/TS)" },
 { value: "ruff", label: "Ruff (Python)" },
 { value: "gofmt", label: "gofmt (Go)" },
 { value: "prettier", label: "Prettier (Web)" },
];

export function AutofixPanel({ workspacePath }: { workspacePath: string | null }) {
 const [detectedFw, setDetectedFw] = useState<string | null>(null);
 const [selectedFw, setSelectedFw] = useState("");
 const [result, setResult] = useState<AutofixResult | null>(null);
 const [running, setRunning] = useState(false);
 const [applying, setApplying] = useState(false);
 const [reverting, setReverting] = useState(false);
 const [error, setError] = useState<string | null>(null);
 const [message, setMessage] = useState<string | null>(null);
 const [showDiff, setShowDiff] = useState(true);
 const cancelRef = useRef(false);
 const taskIdRef = useRef(0);

 useEffect(() => {
 if (!workspacePath) return;
 // Detect framework via existing detect_coverage_tool (same detection logic)
 invoke<string>("detect_coverage_tool", { workspace: workspacePath })
 .then(fw => {
 // Map coverage tool names to autofix framework names
 const map: Record<string, string> = {
 "cargo-llvm-cov": "clippy",
 "nyc": "eslint",
 "npm-coverage": "eslint",
 "coverage.py": "ruff",
 "go-cover": "gofmt",
 };
 const mapped = map[fw] ?? fw;
 setDetectedFw(mapped);
 setSelectedFw(mapped);
 })
 .catch(() => setDetectedFw(null));
 }, [workspacePath]);

 const handleSuspend = () => {
 cancelRef.current = true;
 setRunning(false);
 setError("Autofix suspended by user.");
 };

 const handleRun = async () => {
 if (!workspacePath) return;
 cancelRef.current = false;
 taskIdRef.current += 1;
 const thisId = taskIdRef.current;
 setRunning(true);
 setError(null);
 setResult(null);
 setMessage(null);
 try {
 const r = await invoke<AutofixResult>("run_autofix", {
 workspace: workspacePath,
 framework: selectedFw || null,
 });
 if (cancelRef.current || taskIdRef.current !== thisId) return;
 setResult(r);
 } catch (e: unknown) {
 if (cancelRef.current || taskIdRef.current !== thisId) return;
 setError(String(e));
 } finally {
 if (!cancelRef.current && taskIdRef.current === thisId) {
 setRunning(false);
 }
 }
 };

 const handleApply = async () => {
 if (!workspacePath || !result) return;
 setApplying(true);
 try {
 await invoke("apply_autofix", { workspace: workspacePath, apply: true });
 setMessage(` Applied ${result.files_changed} file changes (staged for commit).`);
 setResult(null);
 } catch (e: unknown) {
 setError(String(e));
 } finally {
 setApplying(false);
 }
 };

 const handleRevert = async () => {
 if (!workspacePath || !result) return;
 setReverting(true);
 try {
 await invoke("apply_autofix", { workspace: workspacePath, apply: false });
 setMessage(" Changes reverted. Working tree restored.");
 setResult(null);
 } catch (e: unknown) {
 setError(String(e));
 } finally {
 setReverting(false);
 }
 };

 const diffLines = result?.diff.split("\n") ?? [];

 return (
 <div style={{ padding: "12px", fontFamily: "monospace", fontSize: "13px", height: "100%", overflowY: "auto" }}>
 <div style={{ fontWeight: "bold", marginBottom: "12px" }}>Codemod & Auto-Fix</div>

 {/* Framework selector */}
 <div style={{ display: "flex", gap: "8px", marginBottom: "12px", flexWrap: "wrap" }}>
 <select
 value={selectedFw}
 onChange={e => setSelectedFw(e.target.value)}
 style={{ background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "4px", padding: "4px 8px", fontFamily: "inherit", fontSize: "12px", flex: 1 }}
 >
 <option value="">Auto-detect</option>
 {FRAMEWORKS.map(fw => (
 <option key={fw.value} value={fw.value}>{fw.label}</option>
 ))}
 </select>
 {detectedFw && (
 <span style={{ color: "var(--text-secondary)", fontSize: "11px", alignSelf: "center" }}>
 detected: {detectedFw}
 </span>
 )}
 {running ? (
 <button
 onClick={handleSuspend}
 style={{
 background: "var(--error-color)",
 color: "white", border: "none", borderRadius: "4px",
 padding: "4px 16px", cursor: "pointer",
 }}
 >
 Suspend
 </button>
 ) : (
 <button
 onClick={handleRun}
 disabled={!workspacePath}
 style={{
 background: "var(--accent-color)",
 color: "white", border: "none", borderRadius: "4px",
 padding: "4px 16px", cursor: !workspacePath ? "default" : "pointer",
 }}
 >
 Run Autofix
 </button>
 )}
 </div>

 {/* Info box */}
 {!result && !running && !error && (
 <div style={{ background: "var(--bg-secondary)", padding: "10px", borderRadius: "4px", color: "var(--text-secondary)", fontSize: "12px", marginBottom: "12px" }}>
 <div style={{ marginBottom: "4px", fontWeight: "bold", color: "var(--text-secondary)" }}>What this does:</div>
 <ul style={{ margin: 0, paddingLeft: "16px", lineHeight: "1.6" }}>
 <li><b>clippy</b>: runs <code>cargo clippy --fix</code></li>
 <li><b>eslint</b>: runs <code>npx eslint --fix .</code></li>
 <li><b>ruff</b>: runs <code>ruff check --fix .</code></li>
 <li><b>gofmt</b>: runs <code>gofmt -w .</code></li>
 <li><b>prettier</b>: runs <code>npx prettier --write .</code></li>
 </ul>
 <div style={{ marginTop: "6px" }}>After running, review the diff and choose Apply or Revert.</div>
 </div>
 )}

 {error && (
 <div style={{ background: "rgba(244,67,54,0.1)", color: "var(--error-color)", padding: "8px", borderRadius: "4px", marginBottom: "12px", whiteSpace: "pre-wrap", fontSize: "12px" }}>
 {error}
 </div>
 )}

 {message && (
 <div style={{ background: "rgba(76,175,80,0.1)", color: "var(--success-color)", padding: "8px", borderRadius: "4px", marginBottom: "12px", fontSize: "12px" }}>
 {message}
 </div>
 )}

 {result && (
 <div>
 {/* Summary */}
 <div style={{ display: "flex", alignItems: "center", gap: "12px", marginBottom: "10px" }}>
 <div style={{ background: "var(--bg-secondary)", padding: "6px 12px", borderRadius: "4px" }}>
 <span style={{ color: result.files_changed > 0 ? "var(--success-color)" : "var(--text-secondary)", fontWeight: "bold" }}>
 {result.files_changed} file{result.files_changed !== 1 ? "s" : ""} changed
 </span>
 <span style={{ color: "var(--text-secondary)", fontSize: "11px", marginLeft: "8px" }}>
 via {result.framework}
 </span>
 </div>
 {result.files_changed > 0 && (
 <>
 <button
 onClick={handleApply}
 disabled={applying}
 style={{ background: "var(--success-color)", color: "var(--text-primary)", border: "none", borderRadius: "4px", padding: "5px 14px", cursor: "pointer", fontSize: "12px" }}
 >
 {applying ? "…" : "✓ Apply & Stage"}
 </button>
 <button
 onClick={handleRevert}
 disabled={reverting}
 style={{ background: "var(--error-color)", color: "var(--text-primary)", border: "none", borderRadius: "4px", padding: "5px 14px", cursor: "pointer", fontSize: "12px" }}
 >
 {reverting ? "…" : "✕ Revert"}
 </button>
 </>
 )}
 <button
 onClick={() => setShowDiff(d => !d)}
 style={{ marginLeft: "auto", background: "var(--bg-secondary)", color: "var(--text-secondary)", border: "1px solid var(--border-color)", borderRadius: "4px", padding: "3px 8px", cursor: "pointer", fontSize: "11px" }}
 >
 {showDiff ? "Hide diff" : "Show diff"}
 </button>
 </div>

 {result.files_changed === 0 && (
 <div style={{ color: "var(--success-color)", fontSize: "12px", marginBottom: "10px" }}>
 ✓ No issues found — code is already clean!
 </div>
 )}

 {showDiff && result.diff && (
 <div style={{ background: "var(--bg-secondary)", borderRadius: "4px", overflow: "auto", maxHeight: "400px", fontSize: "11px" }}>
 {diffLines.map((line, i) => {
 const color = line.startsWith("+") && !line.startsWith("+++") ? "rgba(76,175,80,0.15)" :
 line.startsWith("-") && !line.startsWith("---") ? "rgba(244,67,54,0.15)" :
 line.startsWith("@@") ? "rgba(33,150,243,0.15)" : "transparent";
 const textColor = line.startsWith("+") && !line.startsWith("+++") ? "var(--success-color)" :
 line.startsWith("-") && !line.startsWith("---") ? "var(--error-color)" :
 line.startsWith("@@") ? "var(--accent-color)" :
 line.startsWith("diff ") || line.startsWith("---") || line.startsWith("+++") ? "var(--text-secondary)" :
 "var(--text-secondary)";
 return (
 <div key={i} style={{ background: color, color: textColor, padding: "1px 8px", whiteSpace: "pre", fontFamily: "monospace" }}>
 {line || " "}
 </div>
 );
 })}
 </div>
 )}

 {showDiff && result.stdout && !result.diff && (
 <pre style={{ background: "var(--bg-secondary)", padding: "8px", borderRadius: "4px", fontSize: "11px", overflow: "auto", maxHeight: "300px", whiteSpace: "pre-wrap" }}>
 {result.stdout}
 </pre>
 )}
 </div>
 )}
 </div>
 );
}
