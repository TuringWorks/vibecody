/**
 * DiffReviewPanel — chunk-level diff accept/reject.
 *
 * When the AI agent proposes a file write, this panel displays the diff
 * hunk by hunk. The user can accept or reject each individual hunk and
 * then click "Apply" to write the resulting file, or "Reject All" to cancel.
 *
 * Equivalent to Cursor's per-hunk Accept/Reject feature.
 *
 * Algorithm:
 * 1. Compute a Myers diff between `original` and `modified` (line-level).
 * 2. Group contiguous changed lines into hunks with 3 lines of context each.
 * 3. Render each hunk with Accept ✓ / Reject ✗ toggle buttons.
 * 4. "Apply" assembles the final file: accepted hunks use `modified` lines,
 * rejected hunks revert to `original` lines.
 */

import React, { useState, useMemo, useEffect } from "react";
// lucide-react icons not needed

// ── Diff types ────────────────────────────────────────────────────────────────

type LineKind = "equal" | "insert" | "delete";

interface DiffLine {
 kind: LineKind;
 origLine?: number; // 1-based in original
 modLine?: number; // 1-based in modified
 text: string;
}

interface DiffHunk {
 id: number;
 lines: DiffLine[];
 accepted: boolean; // true = take modified, false = keep original
}

// ── Myers diff (line-level, simple LCS) ──────────────────────────────────────

function diffLines(original: string[], modified: string[]): DiffLine[] {
 const oa = original;
 const mb = modified;
 const n = oa.length;
 const m = mb.length;

 // LCS table
 const dp: number[][] = Array.from({ length: n + 1 }, () => new Array(m + 1).fill(0));
 for (let i = n - 1; i >= 0; i--) {
 for (let j = m - 1; j >= 0; j--) {
 if (oa[i] === mb[j]) {
 dp[i][j] = dp[i + 1][j + 1] + 1;
 } else {
 dp[i][j] = Math.max(dp[i + 1][j], dp[i][j + 1]);
 }
 }
 }

 const result: DiffLine[] = [];
 let i = 0, j = 0;
 let origLine = 1, modLine = 1;

 while (i < n || j < m) {
 if (i < n && j < m && oa[i] === mb[j]) {
 result.push({ kind: "equal", origLine: origLine++, modLine: modLine++, text: oa[i] });
 i++;
 j++;
 } else if (j < m && (i >= n || dp[i][j + 1] >= dp[i + 1][j])) {
 result.push({ kind: "insert", modLine: modLine++, text: mb[j] });
 j++;
 } else {
 result.push({ kind: "delete", origLine: origLine++, text: oa[i] });
 i++;
 }
 }

 return result;
}

// ── Group diff lines into hunks with context ──────────────────────────────────

const CONTEXT = 3;

function buildHunks(diffed: DiffLine[]): Omit<DiffHunk, "accepted">[] {
 // Find changed line indices
 const changedAt = new Set<number>();
 diffed.forEach((line, idx) => {
 if (line.kind !== "equal") changedAt.add(idx);
 });

 if (changedAt.size === 0) return [];

 // Build ranges with context
 const ranges: [number, number][] = [];
 let rangeStart = -1, rangeEnd = -1;

 for (const idx of Array.from(changedAt).sort((a, b) => a - b)) {
 const lo = Math.max(0, idx - CONTEXT);
 const hi = Math.min(diffed.length - 1, idx + CONTEXT);
 if (rangeStart === -1) {
 rangeStart = lo;
 rangeEnd = hi;
 } else if (lo <= rangeEnd + 1) {
 rangeEnd = Math.max(rangeEnd, hi);
 } else {
 ranges.push([rangeStart, rangeEnd]);
 rangeStart = lo;
 rangeEnd = hi;
 }
 }
 if (rangeStart !== -1) ranges.push([rangeStart, rangeEnd]);

 return ranges.map(([start, end], id) => ({
 id,
 lines: diffed.slice(start, end + 1),
 }));
}

// ── Assemble final content from hunks + original ──────────────────────────────

function assembleFinal(
 originalLines: string[],
 _modifiedLines: string[],
 hunks: DiffHunk[],
 _allDiffed: DiffLine[],
): string {
 // For each hunk, track which original line numbers it touches
 // Accept → use modified side; reject → keep original

 // Build a mapping: origLine → replaced text (for accepted hunks)
 const acceptedInserts = new Map<number, string[]>(); // before which orig line to insert
 const deletedOrigLines = new Set<number>(); // orig lines to remove (in accepted hunks)

 for (const hunk of hunks) {
 if (!hunk.accepted) continue;
 // Collect inserts + deletes in this hunk
 let afterOrigLine = 0; // the last equal orig line seen in this hunk
 const insertBuffer: string[] = [];

 for (const line of hunk.lines) {
 if (line.kind === "equal") {
 if (insertBuffer.length > 0 && line.origLine != null) {
 // Flush insert buffer before this equal line
 const key = line.origLine;
 acceptedInserts.set(key, [...(acceptedInserts.get(key) ?? []), ...insertBuffer]);
 insertBuffer.length = 0;
 }
 afterOrigLine = line.origLine ?? afterOrigLine;
 } else if (line.kind === "delete") {
 if (line.origLine != null) deletedOrigLines.add(line.origLine);
 } else if (line.kind === "insert") {
 insertBuffer.push(line.text);
 }
 }

 // Trailing inserts at end of hunk → insert after afterOrigLine
 if (insertBuffer.length > 0) {
 const key = afterOrigLine + 1; // after last seen orig line
 acceptedInserts.set(key, [...(acceptedInserts.get(key) ?? []), ...insertBuffer]);
 }
 }

 const result: string[] = [];
 for (let i = 1; i <= originalLines.length; i++) {
 const before = acceptedInserts.get(i);
 if (before) result.push(...before);
 if (!deletedOrigLines.has(i)) {
 result.push(originalLines[i - 1]);
 }
 }
 // Trailing inserts after last orig line
 const trailing = acceptedInserts.get(originalLines.length + 1);
 if (trailing) result.push(...trailing);

 // If nothing changed (no hunks accepted), return original
 if (deletedOrigLines.size === 0 && acceptedInserts.size === 0) {
 return originalLines.join("\n");
 }

 return result.join("\n");
}

// ── Props ──────────────────────────────────────────────────────────────────────

interface DiffReviewPanelProps {
 /** Original file content. */
 original: string;
 /** AI-proposed modified content. */
 modified: string;
 /** File path (shown in header). */
 filePath: string;
 /** Called with the assembled content (null = reject all / cancel). */
 onApply: (result: string | null) => void;
}

// ── Component ─────────────────────────────────────────────────────────────────

export function DiffReviewPanel({ original, modified, filePath, onApply }: DiffReviewPanelProps) {
 const originalLines = useMemo(() => original.split("\n"), [original]);
 const modifiedLines = useMemo(() => modified.split("\n"), [modified]);
 const allDiffed = useMemo(() => diffLines(originalLines, modifiedLines), [originalLines, modifiedLines]);
 const rawHunks = useMemo(() => buildHunks(allDiffed), [allDiffed]);

 const [hunks, setHunks] = useState<DiffHunk[]>(() =>
 rawHunks.map((h) => ({ ...h, accepted: true }))
 );

 // Sync hunks state when props (original/modified) change after mount
 useEffect(() => {
 setHunks(rawHunks.map((h) => ({ ...h, accepted: true })));
 }, [rawHunks]);

 const noChanges = hunks.length === 0;
 const acceptedCount = hunks.filter((h) => h.accepted).length;

 const toggle = (id: number) => {
 setHunks((prev) =>
 prev.map((h) => (h.id === id ? { ...h, accepted: !h.accepted } : h))
 );
 };

 const acceptAll = () => setHunks((prev) => prev.map((h) => ({ ...h, accepted: true })));
 const rejectAll = () => setHunks((prev) => prev.map((h) => ({ ...h, accepted: false })));

 const handleApply = () => {
 if (noChanges || acceptedCount === 0) {
 onApply(null);
 return;
 }
 const result = assembleFinal(originalLines, modifiedLines, hunks, allDiffed);
 onApply(result);
 };

 return (
 <div style={{
 display: "flex", flexDirection: "column", height: "100%",
 background: "var(--bg-primary)", fontFamily: "monospace",
 }}>
 {/* Header */}
 <div style={{
 display: "flex", alignItems: "center", justifyContent: "space-between",
 padding: "8px 12px", borderBottom: "1px solid var(--border-color)",
 background: "var(--bg-secondary)", flexShrink: 0,
 }}>
 <span style={{ fontSize: 13, fontWeight: 600, color: "var(--text-primary)" }}>
 Review Changes — <code style={{ fontSize: 12 }}>{filePath.split("/").pop()}</code>
 </span>
 <div style={{ display: "flex", gap: 6 }}>
 <span style={{ fontSize: 11, color: "var(--text-secondary)", alignSelf: "center" }}>
 {acceptedCount}/{hunks.length} hunks accepted
 </span>
 <button onClick={acceptAll} style={btnStyle("#2d5a27")}>✓ Accept All</button>
 <button onClick={rejectAll} style={btnStyle("#5a2727")}>✗ Reject All</button>
 <button
 onClick={handleApply}
 style={{
 ...btnStyle("var(--accent-blue, #007acc)"),
 color: "var(--text-primary, #e0e0e0)",
 fontWeight: 600,
 }}
 >
 Apply ({acceptedCount})
 </button>
 <button onClick={() => onApply(null)} style={btnStyle("var(--bg-tertiary)")}>
 Cancel
 </button>
 </div>
 </div>

 {/* Diff body */}
 <div style={{ flex: 1, overflowY: "auto" }}>
 {noChanges ? (
 <div style={{ padding: 24, textAlign: "center", color: "var(--text-secondary)", fontSize: 13 }}>
 No changes detected.
 </div>
 ) : (
 hunks.map((hunk) => (
 <HunkBlock
 key={hunk.id}
 hunk={hunk}
 onToggle={() => toggle(hunk.id)}
 />
 ))
 )}
 </div>
 </div>
 );
}

// ── HunkBlock ─────────────────────────────────────────────────────────────────

function HunkBlock({ hunk, onToggle }: { hunk: DiffHunk; onToggle: () => void }) {
 const insertCount = hunk.lines.filter((l) => l.kind === "insert").length;
 const deleteCount = hunk.lines.filter((l) => l.kind === "delete").length;

 return (
 <div style={{
 margin: "8px 0",
 border: `1px solid ${hunk.accepted ? "var(--border-color)" : "rgba(180,80,80,0.4)"}`,
 borderRadius: 4,
 overflow: "hidden",
 opacity: hunk.accepted ? 1 : 0.6,
 }}>
 {/* Hunk header */}
 <div style={{
 display: "flex", alignItems: "center", gap: 8,
 padding: "4px 10px", background: "var(--bg-secondary)", fontSize: 11,
 }}>
 <button
 onClick={onToggle}
 style={{
 padding: "2px 10px", borderRadius: 3, border: "none",
 background: hunk.accepted ? "#2d5a27" : "#5a2727",
 color: "var(--text-primary, #e0e0e0)", cursor: "pointer", fontWeight: 600, fontSize: 11,
 }}
 >
 {hunk.accepted ? "✓ Accept" : "✗ Reject"}
 </button>
 <span style={{ color: "var(--success-color, #4caf50)" }}>+{insertCount}</span>
 <span style={{ color: "var(--error-color, #f44336)" }}>-{deleteCount}</span>
 </div>

 {/* Hunk lines */}
 <div style={{ fontSize: 12, lineHeight: 1.5 }}>
 {hunk.lines.map((line, idx) => (
 <div
 key={idx}
 style={{
 display: "flex",
 background: line.kind === "insert"
 ? "rgba(40,100,40,0.25)"
 : line.kind === "delete"
 ? "rgba(100,40,40,0.25)"
 : "transparent",
 borderLeft: `3px solid ${
 line.kind === "insert" ? "var(--success-color, #4caf50)"
 : line.kind === "delete" ? "var(--error-color, #f44336)"
 : "transparent"
 }`,
 }}
 >
 {/* Gutter */}
 <span style={{
 width: 70, flexShrink: 0, textAlign: "right", padding: "0 6px",
 color: "var(--text-secondary)", userSelect: "none", fontSize: 10,
 }}>
 {line.origLine ?? ""}
 <span style={{ margin: "0 2px", color: "var(--text-secondary)" }}>
 {line.kind === "insert" ? "+" : line.kind === "delete" ? "-" : " "}
 </span>
 {line.modLine ?? ""}
 </span>
 {/* Content */}
 <pre style={{
 margin: 0, padding: "0 6px", flex: 1,
 whiteSpace: "pre-wrap", wordBreak: "break-all",
 color: line.kind === "insert"
 ? "#7ec87e"
 : line.kind === "delete"
 ? "#e07070"
 : "var(--text-primary)",
 }}>
 {line.text}
 </pre>
 </div>
 ))}
 </div>
 </div>
 );
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function btnStyle(bg: string): React.CSSProperties {
 return {
 padding: "3px 10px", fontSize: 11, borderRadius: 3,
 border: "1px solid var(--border-color)",
 background: bg, color: "var(--text-primary)",
 cursor: "pointer",
 };
}
