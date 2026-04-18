/**
 * DiffToolPanel — Side-by-side text / code diff viewer.
 *
 * Paste or type two blocks of text; get a live line-by-line diff with
 * added/removed/unchanged markers, a unified patch, and copy buttons.
 * Pure TypeScript — no Tauri commands required.
 */
import { useState, useMemo } from "react";
import { Check, X } from "lucide-react";

// ── Myers diff (line-level) ────────────────────────────────────────────────────

type Op = "eq" | "ins" | "del";
interface DiffLine { op: Op; text: string; leftN?: number; rightN?: number; }

function diffLines(a: string, b: string): DiffLine[] {
 const la = a === "" ? [] : a.split("\n");
 const lb = b === "" ? [] : b.split("\n");
 const n = la.length, m = lb.length;
 // LCS-based DP
 const dp: number[][] = Array.from({ length: n + 1 }, () => new Array(m + 1).fill(0));
 for (let i = n - 1; i >= 0; i--)
 for (let j = m - 1; j >= 0; j--)
 dp[i][j] = la[i] === lb[j] ? dp[i + 1][j + 1] + 1 : Math.max(dp[i + 1][j], dp[i][j + 1]);

 const result: DiffLine[] = [];
 let i = 0, j = 0, li = 1, ri = 1;
 while (i < n && j < m) {
 if (la[i] === lb[j]) { result.push({ op: "eq", text: la[i], leftN: li++, rightN: ri++ }); i++; j++; }
 else if (dp[i + 1][j] >= dp[i][j + 1]) { result.push({ op: "del", text: la[i], leftN: li++ }); i++; }
 else { result.push({ op: "ins", text: lb[j], rightN: ri++ }); j++; }
 }
 while (i < n) { result.push({ op: "del", text: la[i++], leftN: li++ }); }
 while (j < m) { result.push({ op: "ins", text: lb[j++], rightN: ri++ }); }
 return result;
}

function buildUnified(diff: DiffLine[], context = 3): string {
 const lines = diff;
 const chunks: number[][] = [];
 let inChunk = false;
 for (let i = 0; i < lines.length; i++) {
 if (lines[i].op !== "eq") {
 const s = Math.max(0, i - context);
 const e = Math.min(lines.length - 1, i + context);
 if (!inChunk || s > chunks[chunks.length - 1][1] + 1) { chunks.push([s, e]); inChunk = true; }
 else { chunks[chunks.length - 1][1] = e; }
 }
 }
 return chunks.map(([s, e]) => {
 const hunk = lines.slice(s, e + 1);
 const leftStart = hunk.find(l => l.leftN)?.leftN ?? 1;
 const rightStart = hunk.find(l => l.rightN)?.rightN ?? 1;
 const delCount = hunk.filter(l => l.op !== "ins").length;
 const insCount = hunk.filter(l => l.op !== "del").length;
 const header = `@@ -${leftStart},${delCount} +${rightStart},${insCount} @@`;
 const body = hunk.map(l => l.op === "eq" ? " " + l.text : l.op === "del" ? "-" + l.text : "+" + l.text).join("\n");
 return header + "\n" + body;
 }).join("\n");
}

// ── Stats ──────────────────────────────────────────────────────────────────────

interface Stats { added: number; removed: number; unchanged: number; }
function calcStats(diff: DiffLine[]): Stats {
 return {
 added: diff.filter(l => l.op === "ins").length,
 removed: diff.filter(l => l.op === "del").length,
 unchanged: diff.filter(l => l.op === "eq").length,
 };
}

// ── Colours ────────────────────────────────────────────────────────────────────

const OP_BG: Record<Op, string> = { eq: "transparent", ins: "rgba(166,227,161,0.12)", del: "rgba(243,139,168,0.12)" };
const OP_FG: Record<Op, string> = { eq: "var(--text-primary)", ins: "var(--success-color)", del: "var(--error-color)" };
const OP_PFX: Record<Op, string> = { eq: " ", ins: "+", del: "-" };

// ── Component ─────────────────────────────────────────────────────────────────

const SAMPLE_A = `function greet(name) {
 console.log("Hello, " + name);
 return true;
}`;

const SAMPLE_B = `function greet(name: string): void {
 console.log(\`Hello, \${name}!\`);
}`;

type ViewMode = "split" | "unified" | "inline";

function LineNumber({ n }: { n?: number }) {
 return (
 <span style={{ minWidth: 36, display: "inline-block", textAlign: "right", paddingRight: 10, color: "var(--text-secondary)", fontSize: "var(--font-size-xs)", userSelect: "none", flexShrink: 0 }}>
 {n ?? ""}
 </span>
 );
}

export function DiffToolPanel() {
 const [left, setLeft] = useState(SAMPLE_A);
 const [right, setRight] = useState(SAMPLE_B);
 const [mode, setMode] = useState<ViewMode>("split");
 const [copied, setCopied] = useState(false);

 const diff = useMemo(() => diffLines(left, right), [left, right]);
 const stats = useMemo(() => calcStats(diff), [diff]);
 const unified = useMemo(() => buildUnified(diff), [diff]);
 const identical = stats.added === 0 && stats.removed === 0;

 const copyUnified = () => {
 navigator.clipboard.writeText(unified);
 setCopied(true);
 setTimeout(() => setCopied(false), 1500);
 };

 const swap = () => { setLeft(right); setRight(left); };
 const clear = () => { setLeft(""); setRight(""); };

 return (
 <div className="panel-container">
 {/* Header */}
 <div className="panel-header" style={{ flexWrap: "wrap" }}>
 <span style={{ fontSize: "var(--font-size-md)", fontWeight: 600 }}>Diff Tool</span>

 {/* Stats */}
 <div style={{ display: "flex", gap: 6 }}>
 {identical
 ? <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-success)", padding: "2px 8px", background: "color-mix(in srgb, var(--accent-green) 10%, transparent)", border: "1px solid var(--success-color)", borderRadius: "var(--radius-md)", display: "inline-flex", alignItems: "center", gap: 3 }}><Check size={10} /> Identical</span>
 : <>
 {stats.added > 0 && <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-success)", padding: "2px 8px", background: "color-mix(in srgb, var(--accent-green) 10%, transparent)", border: "1px solid var(--success-color)", borderRadius: "var(--radius-md)" }}>+{stats.added}</span>}
 {stats.removed > 0 && <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-danger)", padding: "2px 8px", background: "color-mix(in srgb, var(--accent-rose) 10%, transparent)", border: "1px solid var(--error-color)", borderRadius: "var(--radius-md)" }}>−{stats.removed}</span>}
 <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", padding: "2px 8px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-md)" }}>{stats.unchanged} unchanged</span>
 </>
 }
 </div>

 <div style={{ flex: 1 }} />

 {/* View mode */}
 {(["split", "inline", "unified"] as ViewMode[]).map(v => (
 <button key={v} onClick={() => setMode(v)} style={{ padding: "2px 12px", fontSize: "var(--font-size-xs)", borderRadius: "var(--radius-md)", background: mode === v ? "color-mix(in srgb, var(--accent-blue) 20%, transparent)" : "var(--bg-primary)", border: `1px solid ${mode === v ? "var(--accent-color)" : "var(--border-color)"}`, color: mode === v ? "var(--info-color)" : "var(--text-secondary)", cursor: "pointer", fontWeight: mode === v ? 700 : 400 }}>
 {v}
 </button>
 ))}

 <button className="panel-btn" onClick={swap} style={{ fontSize: "var(--font-size-xs)", padding: "3px 12px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-secondary)", cursor: "pointer" }}>⇄ Swap</button>
 <button className="panel-btn" onClick={clear} style={{ fontSize: "var(--font-size-xs)", padding: "3px 12px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-secondary)", cursor: "pointer", display: "inline-flex", alignItems: "center", gap: 3 }}><X size={10} /> Clear</button>
 {mode === "unified" && (
 <button className="panel-btn" onClick={copyUnified} style={{ fontSize: "var(--font-size-xs)", padding: "3px 12px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-secondary)", cursor: "pointer" }}>
 {copied ? <><Check size={10} /> Copied</> : "Copy patch"}
 </button>
 )}
 </div>

 {/* Input editors (always visible for split/inline, hidden for unified) */}
 {mode !== "unified" && (
 <div style={{ display: "flex", borderBottom: "1px solid var(--border-color)", height: 180, flexShrink: 0 }}>
 {[{ label: "Original (A)", value: left, setter: setLeft }, { label: "Modified (B)", value: right, setter: setRight }].map(({ label, value, setter }, idx) => (
 <div key={idx} style={{ flex: 1, display: "flex", flexDirection: "column", borderRight: idx === 0 ? "1px solid var(--border-color)" : "none" }}>
 <div style={{ padding: "4px 12px", background: "var(--bg-secondary)", fontSize: "var(--font-size-xs)", fontWeight: 600, color: "var(--text-secondary)", borderBottom: "1px solid var(--border-color)", display: "flex", justifyContent: "space-between" }}>
 <span>{label}</span>
 <button onClick={() => { navigator.clipboard.readText().then(t => setter(t)).catch(() => {}); }} style={{ fontSize: 9, background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer" }}>Paste</button>
 </div>
 <textarea
 value={value}
 onChange={e => setter(e.target.value)}
 spellCheck={false}
 style={{ flex: 1, resize: "none", padding: "8px 12px", fontSize: "var(--font-size-base)", fontFamily: "var(--font-mono)", lineHeight: 1.6, background: "var(--bg-primary)", color: "var(--text-primary)", border: "none", outline: "none" }}
 />
 </div>
 ))}
 </div>
 )}

 {/* Diff output */}
 <div style={{ flex: 1, overflow: "auto", fontFamily: "var(--font-mono)", fontSize: "var(--font-size-base)" }}>

 {/* Unified patch */}
 {mode === "unified" && (
 <div style={{ padding: "12px 12px", display: "flex", flexDirection: "column", gap: 4 }}>
 <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
 {[{ label: "Original (A)", value: left, setter: setLeft }, { label: "Modified (B)", value: right, setter: setRight }].map(({ label, value, setter }, idx) => (
 <div key={idx} style={{ flex: 1, display: "flex", flexDirection: "column", gap: 4 }}>
 <label style={{ fontSize: "var(--font-size-xs)", fontWeight: 600, color: "var(--text-secondary)" }}>{label}</label>
 <textarea value={value} onChange={e => setter(e.target.value)} rows={5} spellCheck={false} style={{ resize: "vertical", padding: "8px 8px", fontSize: "var(--font-size-sm)", fontFamily: "var(--font-mono)", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", outline: "none" }} />
 </div>
 ))}
 </div>
 <pre style={{ margin: 0, padding: "12px 16px", background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)", lineHeight: 1.6, color: "var(--text-primary)", overflowX: "auto", whiteSpace: "pre" }}>
 {unified || "(no differences)"}
 </pre>
 </div>
 )}

 {/* Split view */}
 {mode === "split" && (
 <div style={{ display: "flex" }}>
 {/* Left (original) */}
 <div style={{ flex: 1, borderRight: "1px solid var(--border-color)" }}>
 {diff.filter(l => l.op !== "ins").map((l, i) => (
 <div key={i} style={{ display: "flex", background: OP_BG[l.op], borderBottom: "1px solid rgba(255,255,255,0.02)", minHeight: 22 }}>
 <LineNumber n={l.leftN} />
 <span style={{ color: l.op === "del" ? OP_FG.del : OP_FG.eq, fontSize: "var(--font-size-xs)", paddingRight: 6, flexShrink: 0, fontWeight: 700 }}>{l.op === "del" ? "−" : " "}</span>
 <span style={{ flex: 1, color: OP_FG[l.op], padding: "2px 0", whiteSpace: "pre-wrap", wordBreak: "break-all" }}>{l.text}</span>
 </div>
 ))}
 </div>
 {/* Right (modified) */}
 <div style={{ flex: 1 }}>
 {diff.filter(l => l.op !== "del").map((l, i) => (
 <div key={i} style={{ display: "flex", background: OP_BG[l.op], borderBottom: "1px solid rgba(255,255,255,0.02)", minHeight: 22 }}>
 <LineNumber n={l.rightN} />
 <span style={{ color: l.op === "ins" ? OP_FG.ins : OP_FG.eq, fontSize: "var(--font-size-xs)", paddingRight: 6, flexShrink: 0, fontWeight: 700 }}>{l.op === "ins" ? "+" : " "}</span>
 <span style={{ flex: 1, color: OP_FG[l.op], padding: "2px 0", whiteSpace: "pre-wrap", wordBreak: "break-all" }}>{l.text}</span>
 </div>
 ))}
 </div>
 </div>
 )}

 {/* Inline view */}
 {mode === "inline" && (
 <div>
 {diff.map((l, i) => (
 <div key={i} style={{ display: "flex", background: OP_BG[l.op], borderBottom: "1px solid rgba(255,255,255,0.02)", minHeight: 22 }}>
 <LineNumber n={l.leftN} />
 <LineNumber n={l.rightN} />
 <span style={{ color: OP_FG[l.op], fontSize: "var(--font-size-xs)", paddingRight: 8, flexShrink: 0, fontWeight: 700, minWidth: 12 }}>{OP_PFX[l.op]}</span>
 <span style={{ flex: 1, color: OP_FG[l.op], padding: "2px 0", whiteSpace: "pre-wrap", wordBreak: "break-all" }}>{l.text}</span>
 </div>
 ))}
 </div>
 )}
 </div>
 </div>
 );
}
