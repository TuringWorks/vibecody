/**
 * DiffToolPanel — Side-by-side text / code diff viewer.
 *
 * Paste or type two blocks of text; get a live line-by-line diff with
 * added/removed/unchanged markers, a unified patch, and copy buttons.
 * Pure TypeScript — no Tauri commands required.
 */
import { useState, useMemo } from "react";

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
    if (la[i] === lb[j]) { result.push({ op: "eq",  text: la[i], leftN: li++, rightN: ri++ }); i++; j++; }
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
    const leftStart  = hunk.find(l => l.leftN)?.leftN  ?? 1;
    const rightStart = hunk.find(l => l.rightN)?.rightN ?? 1;
    const delCount   = hunk.filter(l => l.op !== "ins").length;
    const insCount   = hunk.filter(l => l.op !== "del").length;
    const header = `@@ -${leftStart},${delCount} +${rightStart},${insCount} @@`;
    const body = hunk.map(l => l.op === "eq" ? " " + l.text : l.op === "del" ? "-" + l.text : "+" + l.text).join("\n");
    return header + "\n" + body;
  }).join("\n");
}

// ── Stats ──────────────────────────────────────────────────────────────────────

interface Stats { added: number; removed: number; unchanged: number; }
function calcStats(diff: DiffLine[]): Stats {
  return {
    added:     diff.filter(l => l.op === "ins").length,
    removed:   diff.filter(l => l.op === "del").length,
    unchanged: diff.filter(l => l.op === "eq").length,
  };
}

// ── Colours ────────────────────────────────────────────────────────────────────

const OP_BG: Record<Op, string>   = { eq: "transparent", ins: "rgba(166,227,161,0.12)", del: "rgba(243,139,168,0.12)" };
const OP_FG: Record<Op, string>   = { eq: "var(--text-primary)", ins: "#a6e3a1", del: "#f38ba8" };
const OP_PFX: Record<Op, string>  = { eq: " ", ins: "+", del: "-" };

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
    <span style={{ minWidth: 36, display: "inline-block", textAlign: "right", paddingRight: 10, color: "var(--text-muted)", fontSize: 10, userSelect: "none", flexShrink: 0 }}>
      {n ?? ""}
    </span>
  );
}

export function DiffToolPanel() {
  const [left, setLeft]   = useState(SAMPLE_A);
  const [right, setRight] = useState(SAMPLE_B);
  const [mode, setMode]   = useState<ViewMode>("split");
  const [copied, setCopied] = useState(false);

  const diff    = useMemo(() => diffLines(left, right), [left, right]);
  const stats   = useMemo(() => calcStats(diff), [diff]);
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
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      {/* Header */}
      <div style={{ padding: "8px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 10, alignItems: "center", flexWrap: "wrap" }}>
        <span style={{ fontSize: 13, fontWeight: 600 }}>⬛ Diff Tool</span>

        {/* Stats */}
        <div style={{ display: "flex", gap: 6 }}>
          {identical
            ? <span style={{ fontSize: 10, color: "#a6e3a1", padding: "2px 8px", background: "rgba(166,227,161,0.1)", border: "1px solid #a6e3a1", borderRadius: 10 }}>✓ Identical</span>
            : <>
                {stats.added   > 0 && <span style={{ fontSize: 10, color: "#a6e3a1", padding: "2px 8px", background: "rgba(166,227,161,0.1)", border: "1px solid #a6e3a1", borderRadius: 10 }}>+{stats.added}</span>}
                {stats.removed > 0 && <span style={{ fontSize: 10, color: "#f38ba8", padding: "2px 8px", background: "rgba(243,139,168,0.1)", border: "1px solid #f38ba8", borderRadius: 10 }}>−{stats.removed}</span>}
                <span style={{ fontSize: 10, color: "var(--text-muted)", padding: "2px 8px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 10 }}>{stats.unchanged} unchanged</span>
              </>
          }
        </div>

        <div style={{ flex: 1 }} />

        {/* View mode */}
        {(["split", "inline", "unified"] as ViewMode[]).map(v => (
          <button key={v} onClick={() => setMode(v)} style={{ padding: "2px 10px", fontSize: 10, borderRadius: 10, background: mode === v ? "rgba(99,102,241,0.2)" : "var(--bg-primary)", border: `1px solid ${mode === v ? "#6366f1" : "var(--border-color)"}`, color: mode === v ? "#89b4fa" : "var(--text-muted)", cursor: "pointer", fontWeight: mode === v ? 700 : 400 }}>
            {v}
          </button>
        ))}

        <button onClick={swap}  style={{ fontSize: 10, padding: "3px 10px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-muted)", cursor: "pointer" }}>⇄ Swap</button>
        <button onClick={clear} style={{ fontSize: 10, padding: "3px 10px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-muted)", cursor: "pointer" }}>✕ Clear</button>
        {mode === "unified" && (
          <button onClick={copyUnified} style={{ fontSize: 10, padding: "3px 10px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-muted)", cursor: "pointer" }}>
            {copied ? "✓ Copied" : "📋 Copy patch"}
          </button>
        )}
      </div>

      {/* Input editors (always visible for split/inline, hidden for unified) */}
      {mode !== "unified" && (
        <div style={{ display: "flex", borderBottom: "1px solid var(--border-color)", height: 180, flexShrink: 0 }}>
          {[{ label: "Original (A)", value: left, setter: setLeft }, { label: "Modified (B)", value: right, setter: setRight }].map(({ label, value, setter }, idx) => (
            <div key={idx} style={{ flex: 1, display: "flex", flexDirection: "column", borderRight: idx === 0 ? "1px solid var(--border-color)" : "none" }}>
              <div style={{ padding: "4px 10px", background: "var(--bg-secondary)", fontSize: 10, fontWeight: 600, color: "var(--text-muted)", borderBottom: "1px solid var(--border-color)", display: "flex", justifyContent: "space-between" }}>
                <span>{label}</span>
                <button onClick={() => { navigator.clipboard.readText().then(t => setter(t)).catch(() => {}); }} style={{ fontSize: 9, background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer" }}>📋 Paste</button>
              </div>
              <textarea
                value={value}
                onChange={e => setter(e.target.value)}
                spellCheck={false}
                style={{ flex: 1, resize: "none", padding: "8px 10px", fontSize: 12, fontFamily: "monospace", lineHeight: 1.6, background: "var(--bg-primary)", color: "var(--text-primary)", border: "none", outline: "none" }}
              />
            </div>
          ))}
        </div>
      )}

      {/* Diff output */}
      <div style={{ flex: 1, overflow: "auto", fontFamily: "monospace", fontSize: 12 }}>

        {/* Unified patch */}
        {mode === "unified" && (
          <div style={{ padding: "10px 12px", display: "flex", flexDirection: "column", gap: 4 }}>
            <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
              {[{ label: "Original (A)", value: left, setter: setLeft }, { label: "Modified (B)", value: right, setter: setRight }].map(({ label, value, setter }, idx) => (
                <div key={idx} style={{ flex: 1, display: "flex", flexDirection: "column", gap: 4 }}>
                  <label style={{ fontSize: 10, fontWeight: 600, color: "var(--text-muted)" }}>{label}</label>
                  <textarea value={value} onChange={e => setter(e.target.value)} rows={5} spellCheck={false} style={{ resize: "vertical", padding: "6px 8px", fontSize: 11, fontFamily: "monospace", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none" }} />
                </div>
              ))}
            </div>
            <pre style={{ margin: 0, padding: "12px 14px", background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)", fontSize: 12, lineHeight: 1.6, color: "var(--text-primary)", overflowX: "auto", whiteSpace: "pre" }}>
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
                  <span style={{ color: l.op === "del" ? OP_FG.del : OP_FG.eq, fontSize: 10, paddingRight: 6, flexShrink: 0, fontWeight: 700 }}>{l.op === "del" ? "−" : " "}</span>
                  <span style={{ flex: 1, color: OP_FG[l.op], padding: "2px 0", whiteSpace: "pre-wrap", wordBreak: "break-all" }}>{l.text}</span>
                </div>
              ))}
            </div>
            {/* Right (modified) */}
            <div style={{ flex: 1 }}>
              {diff.filter(l => l.op !== "del").map((l, i) => (
                <div key={i} style={{ display: "flex", background: OP_BG[l.op], borderBottom: "1px solid rgba(255,255,255,0.02)", minHeight: 22 }}>
                  <LineNumber n={l.rightN} />
                  <span style={{ color: l.op === "ins" ? OP_FG.ins : OP_FG.eq, fontSize: 10, paddingRight: 6, flexShrink: 0, fontWeight: 700 }}>{l.op === "ins" ? "+" : " "}</span>
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
                <span style={{ color: OP_FG[l.op], fontSize: 10, paddingRight: 8, flexShrink: 0, fontWeight: 700, minWidth: 12 }}>{OP_PFX[l.op]}</span>
                <span style={{ flex: 1, color: OP_FG[l.op], padding: "2px 0", whiteSpace: "pre-wrap", wordBreak: "break-all" }}>{l.text}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
