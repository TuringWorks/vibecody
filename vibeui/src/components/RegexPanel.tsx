/**
 * RegexPanel — Live regex tester.
 *
 * Enter a pattern + flags; the test string is highlighted in real-time.
 * Shows per-match details (index, position, capture groups) and a replace
 * preview. Includes a built-in library of common patterns.
 * Pure TypeScript — no Tauri commands required.
 */
import { useState, useMemo } from "react";

// ── Common patterns library ────────────────────────────────────────────────────

interface PatternEntry { name: string; pattern: string; flags: string; description: string; }

const COMMON_PATTERNS: PatternEntry[] = [
 { name: "Email", pattern: "[a-zA-Z0-9._%+\\-]+@[a-zA-Z0-9.\\-]+\\.[a-zA-Z]{2,}", flags: "gi", description: "Standard email address" },
 { name: "URL", pattern: "https?://[^\\s/$.?#].[^\\s]*", flags: "gi", description: "HTTP/HTTPS URL" },
 { name: "IPv4", pattern: "\\b(?:\\d{1,3}\\.){3}\\d{1,3}\\b", flags: "g", description: "IPv4 address" },
 { name: "IPv6", pattern: "([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}", flags: "gi", description: "Full IPv6 address" },
 { name: "Phone (US)", pattern: "\\+?1?[-.\\s]?\\(?\\d{3}\\)?[-.\\s]?\\d{3}[-.\\s]?\\d{4}", flags: "g", description: "US phone number" },
 { name: "Date ISO", pattern: "\\d{4}-(?:0[1-9]|1[0-2])-(?:0[1-9]|[12]\\d|3[01])", flags: "g", description: "YYYY-MM-DD" },
 { name: "Time 24h", pattern: "(?:[01]\\d|2[0-3]):[0-5]\\d(?::[0-5]\\d)?", flags: "g", description: "HH:MM or HH:MM:SS" },
 { name: "Hex Color", pattern: "#(?:[0-9a-fA-F]{3}){1,2}\\b", flags: "gi", description: "#RGB or #RRGGBB" },
 { name: "UUID", pattern: "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}", flags: "gi", description: "UUID v4" },
 { name: "JWT", pattern: "eyJ[A-Za-z0-9_-]+\\.eyJ[A-Za-z0-9_-]+\\.[A-Za-z0-9_-]+", flags: "g", description: "JSON Web Token" },
 { name: "Semver", pattern: "\\bv?(?:0|[1-9]\\d*)\\.(?:0|[1-9]\\d*)\\.(?:0|[1-9]\\d*)(?:-[\\w.-]+)?(?:\\+[\\w.-]+)?\\b", flags: "g", description: "Semantic version" },
 { name: "Git SHA", pattern: "\\b[0-9a-f]{7,40}\\b", flags: "g", description: "Git commit hash (7–40 hex chars)" },
 { name: "Credit Card", pattern: "\\b(?:4\\d{12}(?:\\d{3})?|5[1-5]\\d{14}|3[47]\\d{13}|6(?:011|5\\d{2})\\d{12})\\b", flags: "g", description: "Visa/MC/Amex/Discover" },
 { name: "HTML Tag", pattern: "<\\/?[a-z][a-z0-9]*(?:\\s[^>]*)?>", flags: "gi", description: "Any HTML element tag" },
 { name: "JSON String", pattern: "\"(?:[^\"\\\\]|\\\\.)*\"", flags: "g", description: "Quoted JSON string value" },
 { name: "Markdown Link", pattern: "\\[([^\\]]+)\\]\\(([^)]+)\\)", flags: "g", description: "[text](url)" },
 { name: "Env Var", pattern: "\\$[A-Z_][A-Z0-9_]*|\\$\\{[A-Z_][A-Z0-9_]*\\}", flags: "g", description: "$VAR or ${VAR}" },
 { name: "Line Comment", pattern: "\\/\\/.*$", flags: "gm", description: "Single-line // comment" },
 { name: "Numbers", pattern: "-?\\b\\d+(?:\\.\\d+)?(?:[eE][+-]?\\d+)?\\b", flags: "g", description: "Integer or float" },
];

// ── Types ──────────────────────────────────────────────────────────────────────

interface MatchInfo {
 index: number;
 start: number;
 end: number;
 value: string;
 groups: (string | undefined)[];
 namedGroups: Record<string, string | undefined> | null;
}

// ── Highlight helper ───────────────────────────────────────────────────────────

interface Segment { text: string; highlighted: boolean; matchIndex: number }

function buildSegments(text: string, matches: MatchInfo[]): Segment[] {
 const segs: Segment[] = [];
 let cursor = 0;
 for (const m of matches) {
 if (m.start > cursor) segs.push({ text: text.slice(cursor, m.start), highlighted: false, matchIndex: -1 });
 segs.push({ text: text.slice(m.start, m.end), highlighted: true, matchIndex: m.index });
 cursor = m.end;
 }
 if (cursor < text.length) segs.push({ text: text.slice(cursor), highlighted: false, matchIndex: -1 });
 return segs;
}

// ── Match colours (cycle through 6) ───────────────────────────────────────────

const MATCH_COLOURS = [
 "rgba(137,180,250,0.25)", "rgba(166,227,161,0.25)", "rgba(250,179,135,0.25)",
 "rgba(203,166,247,0.25)", "rgba(249,226,175,0.25)", "rgba(243,139,168,0.25)",
];
const MATCH_BORDERS = ["#89b4fa","#a6e3a1","#fab387","#cba6f7","#f9e2af","#f38ba8"];

// ── Component ──────────────────────────────────────────────────────────────────

const SAMPLE_TEXT = `Contact us at support@example.com or sales@company.org
Visit https://www.example.com/path?q=1#anchor for more.
Call +1 (555) 867-5309 or 555.867.5309
Color codes: #f38ba8, #a6e3a1, #89b4fa
UUID: 550e8400-e29b-41d4-a716-446655440000
Version: v1.2.3-beta+build.42`;

export function RegexPanel() {
 const [pattern, setPattern] = useState("[a-zA-Z0-9._%+\\-]+@[a-zA-Z0-9.\\-]+\\.[a-zA-Z]{2,}");
 const [flags, setFlags] = useState("gi");
 const [testText, setTestText] = useState(SAMPLE_TEXT);
 const [replaceStr, setReplaceStr] = useState("[EMAIL]");
 const [showReplace, setShowReplace] = useState(false);
 const [activeLib, setActiveLib] = useState<number | null>(null);

 // ── Compile regex ────────────────────────────────────────────────────────────

 const { regex, error } = useMemo(() => {
 if (!pattern) return { regex: null, error: null };
 try {
 return { regex: new RegExp(pattern, flags), error: null };
 } catch (e) {
 return { regex: null, error: (e as Error).message };
 }
 }, [pattern, flags]);

 // ── Run matches ─────────────────────────────────────────────────────────────

 const matches = useMemo<MatchInfo[]>(() => {
 if (!regex || !testText) return [];
 const result: MatchInfo[] = [];
 const r = new RegExp(regex.source, regex.flags.includes("g") ? regex.flags : regex.flags + "g");
 let m: RegExpExecArray | null;
 let safety = 0;
 while ((m = r.exec(testText)) !== null && safety++ < 2000) {
 result.push({
 index: result.length,
 start: m.index,
 end: m.index + m[0].length,
 value: m[0],
 groups: m.slice(1),
 namedGroups: m.groups ? { ...m.groups } : null,
 });
 if (!r.flags.includes("g")) break;
 if (m[0].length === 0) r.lastIndex++;
 }
 return result;
 }, [regex, testText]);

 const segments = useMemo(() => buildSegments(testText, matches), [testText, matches]);

 const replacePreview = useMemo(() => {
 if (!regex || !showReplace) return "";
 try { return testText.replace(regex, replaceStr); } catch { return ""; }
 }, [regex, testText, replaceStr, showReplace]);

 // ── Flag toggle ──────────────────────────────────────────────────────────────

 const toggleFlag = (f: string) =>
 setFlags(prev => prev.includes(f) ? prev.replace(f, "") : prev + f);

 // ── Load library entry ───────────────────────────────────────────────────────

 const loadLibEntry = (i: number) => {
 const e = COMMON_PATTERNS[i];
 setPattern(e.pattern);
 setFlags(e.flags);
 setActiveLib(i);
 };

 // ── Render ───────────────────────────────────────────────────────────────────

 return (
 <div style={{ display: "flex", height: "100%", overflow: "hidden" }}>

 {/* Sidebar: common patterns */}
 <div style={{ width: 180, flexShrink: 0, borderRight: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", flexDirection: "column", overflow: "hidden" }}>
 <div style={{ padding: "8px 10px", fontSize: 10, fontWeight: 700, color: "var(--text-muted)", borderBottom: "1px solid var(--border-color)", letterSpacing: "0.05em" }}>COMMON PATTERNS</div>
 <div style={{ flex: 1, overflow: "auto" }}>
 {COMMON_PATTERNS.map((p, i) => (
 <button key={i} onClick={() => loadLibEntry(i)} style={{ display: "block", width: "100%", textAlign: "left", padding: "6px 10px", fontSize: 11, background: activeLib === i ? "rgba(99,102,241,0.15)" : "transparent", color: activeLib === i ? "var(--text-info)" : "var(--text-primary)", border: "none", borderBottom: "1px solid rgba(255,255,255,0.04)", cursor: "pointer", lineHeight: 1.4 }}>
 <div style={{ fontWeight: 600 }}>{p.name}</div>
 <div style={{ fontSize: 9, color: "var(--text-muted)", marginTop: 1 }}>{p.description}</div>
 </button>
 ))}
 </div>
 </div>

 {/* Main area */}
 <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>

 {/* Header */}
 <div style={{ padding: "8px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
 <span style={{ fontSize: 13, fontWeight: 600 }}>Regex Tester</span>
 <div style={{ marginLeft: "auto", display: "flex", gap: 6, alignItems: "center" }}>
 <span style={{ fontSize: 10, color: "var(--text-muted)" }}>matches:</span>
 <span style={{ fontSize: 11, fontWeight: 700, color: matches.length > 0 ? "var(--text-success)" : "var(--text-muted)" }}>{matches.length}</span>
 {error && <span style={{ fontSize: 10, color: "var(--text-danger)", maxWidth: 280, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{error}</span>}
 </div>
 </div>

 {/* Pattern row */}
 <div style={{ padding: "8px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
 {/* Delimiter */}
 <span style={{ fontSize: 16, color: "var(--text-muted)", fontFamily: "var(--font-mono)" }}>/</span>
 <input
 value={pattern}
 onChange={e => { setPattern(e.target.value); setActiveLib(null); }}
 placeholder="pattern"
 spellCheck={false}
 style={{ flex: 1, minWidth: 0, padding: "4px 8px", fontSize: 13, fontFamily: "var(--font-mono)", background: error ? "rgba(243,139,168,0.08)" : "var(--bg-primary)", border: `1px solid ${error ? "var(--text-danger)" : "var(--border-color)"}`, borderRadius: 4, color: "var(--text-primary)", outline: "none" }}
 />
 <span style={{ fontSize: 16, color: "var(--text-muted)", fontFamily: "var(--font-mono)" }}>/</span>
 {/* Flags */}
 {["g","i","m","s","u"].map(f => (
 <button key={f} onClick={() => toggleFlag(f)} style={{ padding: "2px 8px", fontSize: 11, fontFamily: "var(--font-mono)", fontWeight: 700, borderRadius: 4, border: `1px solid ${flags.includes(f) ? "var(--accent-primary)" : "var(--border-color)"}`, background: flags.includes(f) ? "rgba(99,102,241,0.2)" : "var(--bg-primary)", color: flags.includes(f) ? "var(--text-info)" : "var(--text-muted)", cursor: "pointer" }}>{f}</button>
 ))}
 {/* Replace toggle */}
 <button onClick={() => setShowReplace(v => !v)} style={{ padding: "3px 10px", fontSize: 10, borderRadius: 4, border: `1px solid ${showReplace ? "var(--text-warning-alt)" : "var(--border-color)"}`, background: showReplace ? "rgba(250,179,135,0.1)" : "var(--bg-primary)", color: showReplace ? "var(--text-warning-alt)" : "var(--text-muted)", cursor: "pointer" }}>⇄ Replace</button>
 </div>

 {/* Replace row */}
 {showReplace && (
 <div style={{ padding: "6px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 8, alignItems: "center" }}>
 <span style={{ fontSize: 11, color: "var(--text-muted)", flexShrink: 0 }}>Replace with:</span>
 <input value={replaceStr} onChange={e => setReplaceStr(e.target.value)} placeholder="replacement (use $1, $2 for groups)" spellCheck={false} style={{ flex: 1, padding: "3px 8px", fontSize: 12, fontFamily: "var(--font-mono)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none" }} />
 </div>
 )}

 {/* Body: test string + results */}
 <div style={{ flex: 1, overflow: "auto", display: "flex", flexDirection: "column", gap: 0 }}>

 {/* Test string input */}
 <div style={{ borderBottom: "1px solid var(--border-color)" }}>
 <div style={{ padding: "4px 12px", fontSize: 10, fontWeight: 700, color: "var(--text-muted)", background: "var(--bg-secondary)", display: "flex", justifyContent: "space-between" }}>
 <span>TEST STRING</span>
 <button onClick={() => setTestText("")} style={{ fontSize: 9, background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer" }}>✕ Clear</button>
 </div>
 <textarea value={testText} onChange={e => setTestText(e.target.value)} rows={5} spellCheck={false}
 style={{ width: "100%", resize: "vertical", padding: "8px 12px", fontSize: 12, fontFamily: "var(--font-mono)", lineHeight: 1.6, background: "var(--bg-primary)", color: "var(--text-primary)", border: "none", outline: "none", boxSizing: "border-box" }} />
 </div>

 {/* Highlighted preview */}
 <div style={{ borderBottom: "1px solid var(--border-color)" }}>
 <div style={{ padding: "4px 12px", fontSize: 10, fontWeight: 700, color: "var(--text-muted)", background: "var(--bg-secondary)" }}>MATCH PREVIEW</div>
 <div style={{ padding: "8px 12px", fontFamily: "var(--font-mono)", fontSize: 12, lineHeight: 1.8, whiteSpace: "pre-wrap", wordBreak: "break-all", minHeight: 40 }}>
 {testText.length === 0
 ? <span style={{ color: "var(--text-muted)", fontStyle: "italic" }}>Enter test string above…</span>
 : segments.map((seg, i) => (
 <span key={i} style={seg.highlighted ? {
 background: MATCH_COLOURS[seg.matchIndex % MATCH_COLOURS.length],
 border: `1px solid ${MATCH_BORDERS[seg.matchIndex % MATCH_BORDERS.length]}`,
 borderRadius: 2,
 padding: "0 1px",
 } : {}}>
 {seg.text}
 </span>
 ))
 }
 </div>
 </div>

 {/* Replace preview */}
 {showReplace && replacePreview && (
 <div style={{ borderBottom: "1px solid var(--border-color)" }}>
 <div style={{ padding: "4px 12px", fontSize: 10, fontWeight: 700, color: "var(--text-warning-alt)", background: "var(--bg-secondary)", display: "flex", justifyContent: "space-between" }}>
 <span>REPLACE PREVIEW</span>
 <button onClick={() => { navigator.clipboard.writeText(replacePreview); }} style={{ fontSize: 9, background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer" }}>Copy</button>
 </div>
 <pre style={{ margin: 0, padding: "8px 12px", fontFamily: "var(--font-mono)", fontSize: 12, lineHeight: 1.6, whiteSpace: "pre-wrap", wordBreak: "break-all", background: "var(--bg-primary)", color: "var(--text-warning-alt)" }}>{replacePreview}</pre>
 </div>
 )}

 {/* Match list */}
 {matches.length > 0 && (
 <div>
 <div style={{ padding: "4px 12px", fontSize: 10, fontWeight: 700, color: "var(--text-muted)", background: "var(--bg-secondary)" }}>MATCHES ({matches.length})</div>
 {matches.map(m => (
 <div key={m.index} style={{ borderBottom: "1px solid rgba(255,255,255,0.04)", padding: "6px 12px", display: "flex", gap: 10, alignItems: "flex-start", background: m.index % 2 === 0 ? "transparent" : "rgba(255,255,255,0.015)" }}>
 {/* Index badge */}
 <span style={{ fontSize: 9, fontWeight: 700, color: MATCH_BORDERS[m.index % MATCH_BORDERS.length], background: MATCH_COLOURS[m.index % MATCH_COLOURS.length], border: `1px solid ${MATCH_BORDERS[m.index % MATCH_BORDERS.length]}`, borderRadius: 10, padding: "1px 6px", flexShrink: 0, minWidth: 20, textAlign: "center" }}>
 {m.index + 1}
 </span>
 <div style={{ flex: 1, minWidth: 0 }}>
 {/* Match value */}
 <div style={{ fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--text-primary)", wordBreak: "break-all" }}>{m.value || <em style={{ color: "var(--text-muted)" }}>(empty match)</em>}</div>
 {/* Position */}
 <div style={{ fontSize: 10, color: "var(--text-muted)", marginTop: 2 }}>pos {m.start}–{m.end} · len {m.end - m.start}</div>
 {/* Capture groups */}
 {m.groups.length > 0 && (
 <div style={{ marginTop: 4, display: "flex", flexWrap: "wrap", gap: 4 }}>
 {m.groups.map((g, gi) => (
 <span key={gi} style={{ fontSize: 10, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 4, padding: "1px 6px", color: "var(--text-muted)" }}>
 <span style={{ color: "var(--text-info)" }}>${gi + 1}</span> = <span style={{ fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{g ?? <em>undefined</em>}</span>
 </span>
 ))}
 </div>
 )}
 {/* Named groups */}
 {m.namedGroups && Object.keys(m.namedGroups).length > 0 && (
 <div style={{ marginTop: 4, display: "flex", flexWrap: "wrap", gap: 4 }}>
 {Object.entries(m.namedGroups).map(([k, v]) => (
 <span key={k} style={{ fontSize: 10, background: "rgba(99,102,241,0.08)", border: "1px solid var(--accent-primary)", borderRadius: 4, padding: "1px 6px" }}>
 <span style={{ color: "var(--text-info)" }}>{k}</span> = <span style={{ fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{v ?? <em>undefined</em>}</span>
 </span>
 ))}
 </div>
 )}
 </div>
 {/* Copy button */}
 <button onClick={() => navigator.clipboard.writeText(m.value)} style={{ fontSize: 9, background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer", flexShrink: 0 }}></button>
 </div>
 ))}
 </div>
 )}

 {!error && pattern && matches.length === 0 && testText && (
 <div style={{ padding: "20px 12px", textAlign: "center", color: "var(--text-muted)", fontSize: 12 }}>No matches found</div>
 )}
 </div>
 </div>
 </div>
 );
}
