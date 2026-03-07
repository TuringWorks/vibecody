/**
 * EncodingPanel — Developer encoding & hash utilities.
 *
 * Tabs:
 * Base64 : encode / decode (standard + URL-safe)
 * URL : encodeURIComponent / decodeURIComponent
 * HTML : entity encode / decode
 * Hash : SHA-1 / SHA-256 / SHA-512 via Web Crypto
 * Case : camelCase / snake_case / PascalCase / kebab-case / CONSTANT / Title
 * Stats : char, word, line, sentence, byte counts + frequency table
 *
 * Pure TypeScript + Web Crypto — no Tauri commands required.
 */
import { useState, useEffect, useCallback } from "react";

// ── Base64 ─────────────────────────────────────────────────────────────────────

function b64Encode(s: string, urlSafe = false): string {
 try {
 const bytes = new TextEncoder().encode(s);
 let bin = "";
 bytes.forEach(b => (bin += String.fromCharCode(b)));
 const encoded = btoa(bin);
 return urlSafe ? encoded.replace(/\+/g, "-").replace(/\//g, "_").replace(/=/g, "") : encoded;
 } catch { return ""; }
}

function b64Decode(s: string): string {
 try {
 const normalized = s.replace(/-/g, "+").replace(/_/g, "/");
 const padded = normalized + "=".repeat((4 - normalized.length % 4) % 4);
 const bin = atob(padded);
 const bytes = new Uint8Array(bin.length);
 for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
 return new TextDecoder().decode(bytes);
 } catch (e) { return `Error: ${(e as Error).message}`; }
}

// ── URL encoding ───────────────────────────────────────────────────────────────

function urlEncode(s: string): string {
 try { return encodeURIComponent(s); } catch { return ""; }
}
function urlDecode(s: string): string {
 try { return decodeURIComponent(s); } catch (e) { return `Error: ${(e as Error).message}`; }
}

// ── HTML entities ──────────────────────────────────────────────────────────────

const HTML_ENTITIES: [RegExp, string][] = [
 [/&/g, "&amp;"], [/</g, "&lt;"], [/>/g, "&gt;"],
 [/"/g, "&quot;"], [/'/g, "&#39;"],
];
function htmlEncode(s: string): string {
 return HTML_ENTITIES.reduce((acc, [re, ent]) => acc.replace(re, ent), s);
}
function htmlDecode(s: string): string {
 const d: Record<string, string> = { "&amp;": "&", "&lt;": "<", "&gt;": ">", "&quot;": '"', "&#39;": "'", "&nbsp;": " " };
 return s.replace(/&[a-z]+;|&#\d+;/gi, m => d[m] ?? m);
}

// ── Hashing ────────────────────────────────────────────────────────────────────

async function hashText(text: string, algo: string): Promise<string> {
 const buf = await crypto.subtle.digest(algo, new TextEncoder().encode(text));
 return Array.from(new Uint8Array(buf)).map(b => b.toString(16).padStart(2, "0")).join("");
}

// ── Case converters ────────────────────────────────────────────────────────────

function tokenise(s: string): string[] {
 return s
 .replace(/([a-z])([A-Z])/g, "$1 $2") // camel → words
 .replace(/([A-Z]+)([A-Z][a-z])/g, "$1 $2")
 .split(/[\s_\-./\\]+/)
 .map(w => w.trim())
 .filter(Boolean);
}

const cases: { label: string; fn: (s: string) => string }[] = [
 { label: "camelCase", fn: s => { const t = tokenise(s); return t.map((w,i) => i === 0 ? w.toLowerCase() : w[0].toUpperCase() + w.slice(1).toLowerCase()).join(""); } },
 { label: "PascalCase", fn: s => tokenise(s).map(w => w[0].toUpperCase() + w.slice(1).toLowerCase()).join("") },
 { label: "snake_case", fn: s => tokenise(s).map(w => w.toLowerCase()).join("_") },
 { label: "kebab-case", fn: s => tokenise(s).map(w => w.toLowerCase()).join("-") },
 { label: "CONSTANT_CASE",fn: s => tokenise(s).map(w => w.toUpperCase()).join("_") },
 { label: "Title Case", fn: s => tokenise(s).map(w => w[0].toUpperCase() + w.slice(1).toLowerCase()).join(" ") },
 { label: "lowercase", fn: s => s.toLowerCase() },
 { label: "UPPERCASE", fn: s => s.toUpperCase() },
 { label: "dot.case", fn: s => tokenise(s).map(w => w.toLowerCase()).join(".") },
 { label: "path/case", fn: s => tokenise(s).map(w => w.toLowerCase()).join("/") },
];

// ── Stats ──────────────────────────────────────────────────────────────────────

function getStats(s: string) {
 const chars = s.length;
 const bytes = new TextEncoder().encode(s).length;
 const lines = s ? s.split("\n").length : 0;
 const words = s.trim() ? s.trim().split(/\s+/).length : 0;
 const sentences = s.trim() ? (s.match(/[.!?]+/g) || []).length : 0;
 const paragraphs= s.trim() ? s.trim().split(/\n\s*\n/).length : 0;

 // Character frequency (top 10, ignore whitespace)
 const freq: Record<string, number> = {};
 for (const c of s) { if (c.trim()) freq[c] = (freq[c] ?? 0) + 1; }
 const topChars = Object.entries(freq).sort((a, b) => b[1] - a[1]).slice(0, 10);
 const maxFreq = topChars[0]?.[1] ?? 1;

 return { chars, bytes, lines, words, sentences, paragraphs, topChars, maxFreq };
}

// ── Shared ────────────────────────────────────────────────────────────────────

const SAMPLE = "Hello, World! <script>alert('XSS')</script> © 2025";

type SubTab = "base64" | "url" | "html" | "hash" | "case" | "stats";

function CopyButton({ text }: { text: string }) {
 const [copied, setCopied] = useState(false);
 const click = () => { navigator.clipboard.writeText(text); setCopied(true); setTimeout(() => setCopied(false), 1500); };
 return (
 <button onClick={click} style={{ fontSize: 9, padding: "2px 8px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-muted)", cursor: "pointer" }}>
 {copied ? "✓" : ""}
 </button>
 );
}

function OutputRow({ label, value, colour = "var(--text-primary)" }: { label: string; value: string; colour?: string }) {
 return (
 <div style={{ borderBottom: "1px solid var(--border-color)" }}>
 <div style={{ padding: "3px 12px", fontSize: 10, fontWeight: 700, color: "var(--text-muted)", background: "var(--bg-secondary)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
 <span>{label}</span>
 <CopyButton text={value} />
 </div>
 <div style={{ padding: "6px 12px", fontFamily: "monospace", fontSize: 12, color: colour, wordBreak: "break-all", lineHeight: 1.6, background: "var(--bg-primary)" }}>{value || <span style={{ color: "var(--text-muted)", fontStyle: "italic" }}>—</span>}</div>
 </div>
 );
}

// ── Component ──────────────────────────────────────────────name──────────────

export function EncodingPanel() {
 const [subTab, setSubTab] = useState<SubTab>("base64");
 const [input, setInput] = useState(SAMPLE);
 const [hashes, setHashes] = useState<Record<string, string>>({});
 const [urlSafe, setUrlSafe] = useState(false);

 // ── Hash computation (async) ────────────────────────────────────────────────

 useEffect(() => {
 if (subTab !== "hash" || !input) { setHashes({}); return; }
 let cancelled = false;
 Promise.all(
 ["SHA-1", "SHA-256", "SHA-512"].map(a => hashText(input, a).then(h => [a, h] as [string, string]))
 ).then(results => {
 if (!cancelled) setHashes(Object.fromEntries(results));
 });
 return () => { cancelled = true; };
 }, [input, subTab]);

 const pasteClipboard = useCallback(() => {
 navigator.clipboard.readText().then(t => setInput(t)).catch(() => {});
 }, []);

 const stats = subTab === "stats" ? getStats(input) : null;

 const TABS: { id: SubTab; label: string }[] = [
 { id: "base64", label: "Base64" },
 { id: "url", label: "URL" },
 { id: "html", label: "HTML" },
 { id: "hash", label: "Hash" },
 { id: "case", label: "Case" },
 { id: "stats", label: "Stats" },
 ];

 return (
 <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>

 {/* Header */}
 <div style={{ padding: "8px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 6, alignItems: "center", flexWrap: "wrap" }}>
 <span style={{ fontSize: 13, fontWeight: 600 }}>Encoding & Hash</span>
 <div style={{ display: "flex", gap: 4 }}>
 {TABS.map(t => (
 <button key={t.id} onClick={() => setSubTab(t.id)} style={{ padding: "2px 10px", fontSize: 10, borderRadius: 10, background: subTab === t.id ? "rgba(99,102,241,0.2)" : "var(--bg-primary)", border: `1px solid ${subTab === t.id ? "var(--accent-color, #6366f1)" : "var(--border-color)"}`, color: subTab === t.id ? "var(--info-color, #89b4fa)" : "var(--text-muted)", cursor: "pointer", fontWeight: subTab === t.id ? 700 : 400 }}>{t.label}</button>
 ))}
 </div>
 </div>

 {/* Input */}
 <div style={{ borderBottom: "1px solid var(--border-color)" }}>
 <div style={{ padding: "4px 12px", fontSize: 10, fontWeight: 700, color: "var(--text-muted)", background: "var(--bg-secondary)", display: "flex", gap: 8, alignItems: "center" }}>
 <span>INPUT</span>
 <button onClick={pasteClipboard} style={{ fontSize: 9, background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer" }}>Paste</button>
 <button onClick={() => setInput("")} style={{ fontSize: 9, background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer" }}>✕ Clear</button>
 <span style={{ marginLeft: "auto", fontSize: 9, color: "var(--text-muted)" }}>{input.length} chars</span>
 </div>
 <textarea value={input} onChange={e => setInput(e.target.value)} rows={4} spellCheck={false}
 style={{ width: "100%", resize: "vertical", padding: "8px 12px", fontSize: 12, fontFamily: "monospace", lineHeight: 1.6, background: "var(--bg-primary)", color: "var(--text-primary)", border: "none", outline: "none", boxSizing: "border-box" }} />
 </div>

 {/* Output area */}
 <div style={{ flex: 1, overflow: "auto" }}>

 {/* ── BASE64 ── */}
 {subTab === "base64" && (
 <div>
 <div style={{ padding: "4px 12px", display: "flex", gap: 8, alignItems: "center", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)" }}>
 <label style={{ fontSize: 10, color: "var(--text-muted)", display: "flex", gap: 4, alignItems: "center", cursor: "pointer" }}>
 <input type="checkbox" checked={urlSafe} onChange={e => setUrlSafe(e.target.checked)} style={{ accentColor: "var(--accent-color, #6366f1)" }} />
 URL-safe (no +/=//)
 </label>
 </div>
 <OutputRow label="ENCODED" value={b64Encode(input, urlSafe)} colour="#89b4fa" />
 <OutputRow label="DECODED (treat input as Base64)" value={b64Decode(input)} colour="#a6e3a1" />
 </div>
 )}

 {/* ── URL ── */}
 {subTab === "url" && (
 <div>
 <OutputRow label="URL ENCODED (encodeURIComponent)" value={urlEncode(input)} colour="#fab387" />
 <OutputRow label="URL DECODED (decodeURIComponent)" value={urlDecode(input)} colour="#a6e3a1" />
 </div>
 )}

 {/* ── HTML ── */}
 {subTab === "html" && (
 <div>
 <OutputRow label="HTML ENCODED" value={htmlEncode(input)} colour="#f9e2af" />
 <OutputRow label="HTML DECODED (treat input as HTML-encoded)" value={htmlDecode(input)} colour="#a6e3a1" />
 </div>
 )}

 {/* ── HASH ── */}
 {subTab === "hash" && (
 <div>
 {!input
 ? <div style={{ padding: 16, color: "var(--text-muted)", fontSize: 12 }}>Type or paste text above to compute hashes.</div>
 : ["SHA-1", "SHA-256", "SHA-512"].map(algo => (
 <OutputRow key={algo} label={algo} value={hashes[algo] ?? "computing…"} colour={algo === "SHA-256" ? "#a6e3a1" : algo === "SHA-512" ? "#89b4fa" : "#f38ba8"} />
 ))
 }
 <div style={{ padding: "10px 12px", fontSize: 10, color: "var(--text-muted)", lineHeight: 1.7, background: "var(--bg-secondary)", borderTop: "1px solid var(--border-color)" }}>
 <strong style={{ color: "var(--text-warning-alt, #fab387)" }}>Note:</strong>MD5 is not available in Web Crypto (deprecated for security). Use SHA-256 or higher for any security-sensitive purpose.
 </div>
 </div>
 )}

 {/* ── CASE ── */}
 {subTab === "case" && (
 <div>
 {cases.map(({ label, fn }) => {
 const result = input ? fn(input) : "";
 return (
 <div key={label} style={{ borderBottom: "1px solid var(--border-color)", display: "flex", alignItems: "center" }}>
 <span style={{ width: 140, flexShrink: 0, padding: "6px 12px", fontSize: 10, fontWeight: 700, color: "var(--text-muted)", fontFamily: "monospace", background: "var(--bg-secondary)", alignSelf: "stretch", display: "flex", alignItems: "center" }}>{label}</span>
 <div style={{ flex: 1, padding: "6px 12px", fontFamily: "monospace", fontSize: 12, color: "var(--text-primary)", wordBreak: "break-all" }}>{result || <span style={{ color: "var(--text-muted)", fontStyle: "italic" }}>—</span>}</div>
 {result && (
 <div style={{ paddingRight: 10, flexShrink: 0 }}>
 <button onClick={() => setInput(result)} style={{ fontSize: 9, padding: "2px 6px", background: "none", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-muted)", cursor: "pointer", marginRight: 4 }}>↑ Use</button>
 <CopyButton text={result} />
 </div>
 )}
 </div>
 );
 })}
 </div>
 )}

 {/* ── STATS ── */}
 {subTab === "stats" && stats && (
 <div style={{ padding: "12px" }}>
 {/* Stat cards */}
 <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(110px, 1fr))", gap: 8, marginBottom: 16 }}>
 {[
 { label: "Characters", value: stats.chars, colour: "#89b4fa" },
 { label: "Bytes (UTF-8)", value: stats.bytes, colour: "#cba6f7" },
 { label: "Words", value: stats.words, colour: "#a6e3a1" },
 { label: "Lines", value: stats.lines, colour: "#fab387" },
 { label: "Sentences", value: stats.sentences, colour: "#f9e2af" },
 { label: "Paragraphs", value: stats.paragraphs, colour: "#f38ba8" },
 ].map(({ label, value, colour }) => (
 <div key={label} style={{ background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 6, padding: "8px 10px", textAlign: "center" }}>
 <div style={{ fontSize: 20, fontWeight: 700, color: colour, fontFamily: "monospace" }}>{value.toLocaleString()}</div>
 <div style={{ fontSize: 10, color: "var(--text-muted)", marginTop: 2 }}>{label}</div>
 </div>
 ))}
 </div>

 {/* Char frequency */}
 {stats.topChars.length > 0 && (
 <div>
 <div style={{ fontSize: 10, fontWeight: 700, color: "var(--text-muted)", marginBottom: 8, letterSpacing: "0.05em" }}>TOP CHARACTERS (excluding whitespace)</div>
 <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
 {stats.topChars.map(([ch, count]) => (
 <div key={ch} style={{ display: "flex", alignItems: "center", gap: 8 }}>
 <span style={{ width: 24, textAlign: "center", fontFamily: "monospace", fontSize: 13, fontWeight: 700, color: "var(--text-primary)", flexShrink: 0, background: "var(--bg-secondary)", borderRadius: 4, padding: "1px 0" }}>{ch === " " ? "·" : ch}</span>
 <div style={{ flex: 1, background: "var(--bg-secondary)", borderRadius: 4, height: 12, overflow: "hidden" }}>
 <div style={{ width: `${(count / stats.maxFreq) * 100}%`, height: "100%", background: "rgba(137,180,250,0.5)", transition: "width 0.2s" }} />
 </div>
 <span style={{ fontSize: 11, color: "var(--text-muted)", minWidth: 30, textAlign: "right", fontFamily: "monospace" }}>{count}</span>
 </div>
 ))}
 </div>
 </div>
 )}
 </div>
 )}

 </div>
 </div>
 );
}
