/**
 * UtilitiesPanel — Developer Utilities Swiss Army Knife.
 *
 * 7 sub-tools, all pure TypeScript / browser-native APIs:
 * 1. JWT Inspector – decode header + payload, show expiry
 * 2. JSON Formatter – validate, pretty-print, minify
 * 3. Regex Tester – live match highlighting + named groups
 * 4. Timestamp – unix ↔ ISO ↔ relative
 * 5. Base64 – encode / decode (text & URL-safe)
 * 6. Hash – SHA-256 / SHA-1 / MD5-stub via WebCrypto
 * 7. URL Encode/Decode – encodeURIComponent / query-string parse
 */
import React, { useState, useMemo } from "react";

// ── helpers ─────────────────────────────────────────────────────────────────

function copyText(text: string) {
 navigator.clipboard.writeText(text).catch(() => {});
}

const TOOL_LIST = [
 { id: "jwt", label: "JWT", icon: "" },
 { id: "json", label: "JSON", icon: "{ }" },
 { id: "regex", label: "Regex", icon: ".*" },
 { id: "time", label: "Timestamp", icon: "" },
 { id: "b64", label: "Base64", icon: "B6" },
 { id: "hash", label: "Hash", icon: "#" },
 { id: "url", label: "URL", icon: "link" },
] as const;

type ToolId = typeof TOOL_LIST[number]["id"];

// ── styles ───────────────────────────────────────────────────────────────────

const S = {
 textarea: {
 width: "100%", boxSizing: "border-box" as const,
 padding: "7px 10px", fontSize: 12, fontFamily: "var(--font-mono)",
 background: "var(--bg-primary)", border: "1px solid var(--border-color)",
 borderRadius: 4, color: "var(--text-primary)", outline: "none",
 resize: "vertical" as const, lineHeight: 1.5,
 },
 input: {
 width: "100%", boxSizing: "border-box" as const,
 padding: "6px 10px", fontSize: 12,
 background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
 borderRadius: 4, color: "var(--text-primary)", outline: "none",
 },
 btn: (variant?: "primary" | "danger") => ({
 padding: "5px 14px", fontSize: 11, fontWeight: 600,
 background: variant === "primary" ? "var(--accent-color)"
 : variant === "danger" ? "var(--bg-tertiary)" : "var(--bg-secondary)",
 color: variant === "primary" ? "var(--text-primary)"
 : variant === "danger" ? "var(--error-color)" : "var(--text-secondary)",
 border: variant === "danger" ? "1px solid var(--error-color)" : "1px solid var(--border-color)",
 borderRadius: 4, cursor: "pointer",
 }),
 label: { fontSize: 10, fontWeight: 600 as const, color: "var(--text-muted)", marginBottom: 3, display: "block" as const },
 result: {
 padding: "8px 10px", background: "var(--bg-primary)", borderRadius: 4, fontFamily: "var(--font-mono)",
 fontSize: 11, lineHeight: 1.6, whiteSpace: "pre-wrap" as const,
 border: "1px solid var(--border-color)", wordBreak: "break-all" as const,
 color: "var(--text-primary)", overflowY: "auto" as const, maxHeight: 280,
 },
 error: { color: "var(--text-danger)", fontSize: 11, marginTop: 4 },
 field: { display: "flex", flexDirection: "column" as const, gap: 3 },
};

// ── 1. JWT Inspector ─────────────────────────────────────────────────────────

function b64UrlDecode(s: string) {
 const padded = s.replace(/-/g, "+").replace(/_/g, "/").padEnd(Math.ceil(s.length / 4) * 4, "=");
 try { return JSON.parse(atob(padded)); } catch { return null; }
}

function JwtTool() {
 const [token, setToken] = useState("");
 const parsed = useMemo(() => {
 const parts = token.trim().split(".");
 if (parts.length !== 3) return null;
 const header = b64UrlDecode(parts[0]);
 const payload = b64UrlDecode(parts[1]);
 return header && payload ? { header, payload, sig: parts[2] } : null;
 }, [token]);

 const expiry = parsed?.payload?.exp
 ? new Date(parsed.payload.exp * 1000)
 : null;
 const isExpired = expiry ? expiry < new Date() : false;

 return (
 <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
 <div style={S.field}>
 <label style={S.label}>JWT Token</label>
 <textarea
 rows={3} value={token}
 onChange={(e) => setToken(e.target.value)}
 placeholder="Paste JWT token here…"
 style={S.textarea}
 />
 </div>
 {token && !parsed && (
 <div style={S.error}>Invalid JWT format (expected 3 dot-separated parts)</div>
 )}
 {parsed && (
 <>
 {expiry && (
 <div style={{
 padding: "5px 10px", borderRadius: 4, fontSize: 11, fontWeight: 600,
 background: isExpired ? "color-mix(in srgb, var(--accent-rose) 10%, transparent)" : "color-mix(in srgb, var(--accent-green) 10%, transparent)",
 color: isExpired ? "var(--error-color)" : "var(--success-color)",
 border: `1px solid ${isExpired ? "var(--error-color)" : "var(--success-color)"}`,
 }}>
 {isExpired ? "Expired" : "Valid"} · {expiry.toLocaleString()}
 {parsed.payload.iat && ` · issued ${new Date(parsed.payload.iat * 1000).toLocaleDateString()}`}
 </div>
 )}
 {[["Header", parsed.header], ["Payload", parsed.payload]] .map(([label, data]) => (
 <div key={label as string}>
 <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 4 }}>
 <span style={S.label}>{label as string}</span>
 <button onClick={() => copyText(JSON.stringify(data, null, 2))} style={{ ...S.btn(), fontSize: 10, padding: "2px 8px" }}>Copy</button>
 </div>
 <div style={S.result}>{JSON.stringify(data, null, 2)}</div>
 </div>
 ))}
 </>
 )}
 </div>
 );
}

// ── 2. JSON Formatter ────────────────────────────────────────────────────────

function JsonTool() {
 const [input, setInput] = useState("");
 const [indent, setIndent] = useState(2);
 const [error, setError] = useState<string | null>(null);
 const [output, setOutput] = useState("");

 const format = () => {
 try {
 const parsed = JSON.parse(input);
 setOutput(JSON.stringify(parsed, null, indent));
 setError(null);
 } catch (e) { setError(String(e)); setOutput(""); }
 };
 const minify = () => {
 try {
 setOutput(JSON.stringify(JSON.parse(input)));
 setError(null);
 } catch (e) { setError(String(e)); setOutput(""); }
 };

 return (
 <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
 <div style={S.field}>
 <label style={S.label}>JSON Input</label>
 <textarea rows={6} value={input} onChange={(e) => setInput(e.target.value)} placeholder='{"key": "value"}' style={S.textarea} />
 </div>
 <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
 <button onClick={format} style={S.btn("primary")}>Prettify</button>
 <button onClick={minify} style={S.btn()}>Minify</button>
 <label style={{ ...S.label, marginBottom: 0 }}>Indent:</label>
 <select
 value={indent}
 onChange={(e) => setIndent(Number(e.target.value))}
 style={{ ...S.input, width: 60 }}
 >
 {[2, 4, "\t"].map((v) => <option key={String(v)} value={v === "\t" ? 9 : v as number}>{v === "\t" ? "tab" : v}</option>)}
 </select>
 </div>
 {error && <div style={S.error}> {error}</div>}
 {output && (
 <div>
 <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
 <span style={S.label}>Output</span>
 <button onClick={() => copyText(output)} style={{ ...S.btn(), fontSize: 10, padding: "2px 8px" }}>Copy</button>
 </div>
 <div style={S.result}>{output}</div>
 </div>
 )}
 </div>
 );
}

// ── 3. Regex Tester ──────────────────────────────────────────────────────────

function RegexTool() {
 const [pattern, setPattern] = useState("");
 const [flags, setFlags] = useState("gm");
 const [text, setText] = useState("");

 const { matches, error } = useMemo(() => {
 if (!pattern || !text) return { matches: [], error: null };
 if (pattern.length > 500) return { matches: [], error: "Pattern too long (max 500 characters)" };
 try {
 const re = new RegExp(pattern, flags);
 const matches: RegExpExecArray[] = [];
 let m: RegExpExecArray | null;
 let guard = 0;
 const startTime = performance.now();
 while ((m = re.exec(text)) !== null && ++guard < 500) {
 if (performance.now() - startTime > 100) {
  return { matches: [], error: "Regex execution timed out (>100ms) — pattern may be too complex" };
 }
 matches.push(m);
 if (!flags.includes("g")) break;
 }
 if (performance.now() - startTime > 100) {
 return { matches: [], error: "Regex execution timed out (>100ms) — pattern may be too complex" };
 }
 return { matches, error: null };
 } catch (e) {
 return { matches: [], error: String(e) };
 }
 }, [pattern, flags, text]);

 // Build highlighted React elements (safe — no dangerouslySetInnerHTML)
 const highlightedElements = useMemo(() => {
 if (!matches.length) return [text];
 const parts: React.ReactNode[] = [];
 let cursor = 0;
 let key = 0;
 for (const m of matches) {
 if (m.index > cursor) {
  parts.push(text.slice(cursor, m.index));
 }
 parts.push(
  React.createElement("mark", {
  key: key++,
  style: { background: "var(--warning-color)", color: "var(--bg-tertiary)", borderRadius: 2 },
  }, m[0])
 );
 cursor = m.index + m[0].length;
 }
 if (cursor < text.length) {
 parts.push(text.slice(cursor));
 }
 return parts;
 }, [matches, text]);

 return (
 <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
 <div style={{ display: "flex", gap: 6 }}>
 <div style={{ ...S.field, flex: 1 }}>
 <label style={S.label}>Pattern</label>
 <input value={pattern} onChange={(e) => setPattern(e.target.value)} placeholder="e.g. \b\w+\b" style={S.input} />
 </div>
 <div style={S.field}>
 <label style={S.label}>Flags</label>
 <input value={flags} onChange={(e) => setFlags(e.target.value)} style={{ ...S.input, width: 70 }} />
 </div>
 </div>
 <div style={S.field}>
 <label style={S.label}>Test String</label>
 <textarea rows={5} value={text} onChange={(e) => setText(e.target.value)} placeholder="Text to test against…" style={S.textarea} />
 </div>
 {error && <div style={S.error}> {error}</div>}
 {text && (
 <div>
 <div style={{ ...S.label, marginBottom: 4 }}>
 {matches.length} match{matches.length !== 1 ? "es" : ""}
 </div>
 <div
 style={{ ...S.result, background: "var(--bg-primary)" }}
 >{highlightedElements}</div>
 {matches.length > 0 && matches[0].groups && (
 <div style={{ marginTop: 6 }}>
 <div style={S.label}>Named Groups (first match)</div>
 <div style={S.result}>{JSON.stringify(matches[0].groups, null, 2)}</div>
 </div>
 )}
 </div>
 )}
 </div>
 );
}

// ── 4. Timestamp Converter ───────────────────────────────────────────────────

function TimestampTool() {
 const [value, setValue] = useState(String(Math.floor(Date.now() / 1000)));
 const [mode, setMode] = useState<"unix" | "iso" | "ms">("unix");

 const parsed = useMemo(() => {
 try {
 let d: Date;
 if (mode === "unix") d = new Date(Number(value) * 1000);
 else if (mode === "ms") d = new Date(Number(value));
 else d = new Date(value);
 if (isNaN(d.getTime())) return null;
 return d;
 } catch { return null; }
 }, [value, mode]);

 const now = () => setValue(mode === "ms" ? String(Date.now()) : mode === "unix" ? String(Math.floor(Date.now() / 1000)) : new Date().toISOString());

 return (
 <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
 <div style={{ display: "flex", gap: 6 }}>
 {(["unix", "ms", "iso"] as const).map((m) => (
 <button key={m} onClick={() => setMode(m)} style={{ ...S.btn(mode === m ? "primary" : undefined), fontSize: 11 }}>
 {m === "unix" ? "Unix (s)" : m === "ms" ? "Milliseconds" : "ISO 8601"}
 </button>
 ))}
 <button onClick={now} style={{ ...S.btn(), marginLeft: "auto" }}>Now</button>
 </div>
 <div style={S.field}>
 <label style={S.label}>Input</label>
 <input value={value} onChange={(e) => setValue(e.target.value)} style={S.input} />
 </div>
 {parsed && (
 <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
 {[
 ["Local", parsed.toLocaleString()],
 ["UTC", parsed.toUTCString()],
 ["ISO 8601", parsed.toISOString()],
 ["Unix (s)", String(Math.floor(parsed.getTime() / 1000))],
 ["Unix (ms)", String(parsed.getTime())],
 ["Relative", (() => {
 const diff = (Date.now() - parsed.getTime()) / 1000;
 if (Math.abs(diff) < 60) return `${Math.abs(diff).toFixed(0)}s ${diff > 0 ? "ago" : "from now"}`;
 if (Math.abs(diff) < 3600) return `${(Math.abs(diff)/60).toFixed(0)}m ${diff > 0 ? "ago" : "from now"}`;
 if (Math.abs(diff) < 86400) return `${(Math.abs(diff)/3600).toFixed(1)}h ${diff > 0 ? "ago" : "from now"}`;
 return `${(Math.abs(diff)/86400).toFixed(1)}d ${diff > 0 ? "ago" : "from now"}`;
 })()],
 ].map(([k, v]) => (
 <div key={k} style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "5px 8px", background: "var(--bg-secondary)", borderRadius: 4, border: "1px solid var(--border-color)" }}>
 <span style={{ fontSize: 10, color: "var(--text-muted)", fontWeight: 600, width: 90 }}>{k}</span>
 <span style={{ fontFamily: "var(--font-mono)", fontSize: 11, flex: 1 }}>{v}</span>
 <button onClick={() => copyText(v)} style={{ ...S.btn(), fontSize: 10, padding: "2px 6px" }}>Copy</button>
 </div>
 ))}
 </div>
 )}
 {value && !parsed && <div style={S.error}>Cannot parse "{value}"</div>}
 </div>
 );
}

// ── 5. Base64 ────────────────────────────────────────────────────────────────

function Base64Tool() {
 const [input, setInput] = useState("");
 const [urlSafe, setUrlSafe] = useState(false);

 const encoded = useMemo(() => {
 try {
 const bytes = new TextEncoder().encode(input);
 let r = btoa(String.fromCharCode(...bytes));
 if (urlSafe) r = r.replace(/\+/g, "-").replace(/\//g, "_").replace(/=/g, "");
 return r;
 } catch { return ""; }
 }, [input, urlSafe]);

 const decoded = useMemo(() => {
 try {
 let s = input.replace(/-/g, "+").replace(/_/g, "/");
 const pad = s.length % 4;
 if (pad) s += "=".repeat(4 - pad);
 const binary = atob(s);
 const bytes = Uint8Array.from(binary, (c) => c.charCodeAt(0));
 return new TextDecoder().decode(bytes);
 } catch { return null; }
 }, [input]);

 return (
 <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
 <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
 <label style={{ ...S.label, marginBottom: 0 }}>
 <input type="checkbox" checked={urlSafe} onChange={(e) => setUrlSafe(e.target.checked)} style={{ marginRight: 4 }} />
 URL-safe (no +/= padding)
 </label>
 </div>
 <div style={S.field}>
 <label style={S.label}>Input</label>
 <textarea rows={4} value={input} onChange={(e) => setInput(e.target.value)} placeholder="Enter text or base64…" style={S.textarea} />
 </div>
 <div style={{ display: "flex", gap: 8 }}>
 <div style={{ flex: 1 }}>
 <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 3 }}>
 <span style={S.label}>Encoded</span>
 <button onClick={() => copyText(encoded)} style={{ ...S.btn(), fontSize: 10, padding: "2px 6px" }}>Copy</button>
 </div>
 <div style={{ ...S.result, minHeight: 48 }}>{encoded}</div>
 </div>
 <div style={{ flex: 1 }}>
 <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 3 }}>
 <span style={S.label}>Decoded</span>
 <button onClick={() => copyText(decoded ?? "")} style={{ ...S.btn(), fontSize: 10, padding: "2px 6px" }}>Copy</button>
 </div>
 <div style={{ ...S.result, minHeight: 48, color: decoded ? "var(--text-primary)" : "var(--text-muted)" }}>
 {decoded ?? "Invalid base64"}
 </div>
 </div>
 </div>
 </div>
 );
}

// ── 6. Hash Generator ────────────────────────────────────────────────────────

async function hashText(algo: string, text: string): Promise<string> {
 const buf = await crypto.subtle.digest(algo, new TextEncoder().encode(text));
 return Array.from(new Uint8Array(buf)).map((b) => b.toString(16).padStart(2, "0")).join("");
}

function HashTool() {
 const [input, setInput] = useState("");
 const [hashes, setHashes] = useState<Record<string, string>>({});
 const [loading, setLoading] = useState(false);

 const compute = async () => {
 if (!input) return;
 setLoading(true);
 const results: Record<string, string> = {};
 for (const [label, algo] of [["SHA-256", "SHA-256"], ["SHA-384", "SHA-384"], ["SHA-512", "SHA-512"], ["SHA-1", "SHA-1"]]) {
 try { results[label] = await hashText(algo, input); } catch { results[label] = "error"; }
 }
 setHashes(results);
 setLoading(false);
 };

 return (
 <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
 <div style={S.field}>
 <label style={S.label}>Input Text</label>
 <textarea rows={3} value={input} onChange={(e) => setInput(e.target.value)} placeholder="Text to hash…" style={S.textarea} />
 </div>
 <button onClick={compute} disabled={loading || !input} style={S.btn("primary")}>
 {loading ? "Computing…" : "Compute Hashes"}
 </button>
 {Object.entries(hashes).map(([label, hash]) => (
 <div key={label} style={{ display: "flex", alignItems: "center", gap: 8, padding: "5px 8px", background: "var(--bg-secondary)", borderRadius: 4, border: "1px solid var(--border-color)" }}>
 <span style={{ fontSize: 10, color: "var(--text-muted)", fontWeight: 600, width: 65 }}>{label}</span>
 <span style={{ fontFamily: "var(--font-mono)", fontSize: 10, flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{hash}</span>
 <button onClick={() => copyText(hash)} style={{ ...S.btn(), fontSize: 10, padding: "2px 6px", flexShrink: 0 }}>Copy</button>
 </div>
 ))}
 </div>
 );
}

// ── 7. URL Encoder ───────────────────────────────────────────────────────────

function UrlTool() {
 const [input, setInput] = useState("");
 const encoded = useMemo(() => { try { return encodeURIComponent(input); } catch { return ""; } }, [input]);
 const decoded = useMemo(() => { try { return decodeURIComponent(input); } catch { return null; } }, [input]);

 const params = useMemo(() => {
 try {
 const u = new URL(input.includes("://") ? input : `https://x.com?${input}`);
 const p: [string, string][] = [];
 u.searchParams.forEach((v, k) => p.push([k, v]));
 return p;
 } catch { return []; }
 }, [input]);

 return (
 <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
 <div style={S.field}>
 <label style={S.label}>Input (URL, query string, or plain text)</label>
 <textarea rows={3} value={input} onChange={(e) => setInput(e.target.value)} placeholder="https://example.com/path?foo=bar&baz=qux" style={S.textarea} />
 </div>
 {[
 ["Encoded", encoded],
 ["Decoded", decoded ?? "Invalid percent-encoding"],
 ].map(([label, val]) => (
 <div key={label as string}>
 <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 3 }}>
 <span style={S.label}>{label as string}</span>
 <button onClick={() => copyText(val as string)} style={{ ...S.btn(), fontSize: 10, padding: "2px 6px" }}>Copy</button>
 </div>
 <div style={{ ...S.result, minHeight: 32 }}>{val as string}</div>
 </div>
 ))}
 {params.length > 0 && (
 <div>
 <div style={S.label}>Query Params ({params.length})</div>
 <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
 {params.map(([k, v]) => (
 <div key={k} style={{ display: "flex", gap: 8, padding: "4px 8px", background: "var(--bg-secondary)", borderRadius: 4, border: "1px solid var(--border-color)", fontSize: 11, fontFamily: "var(--font-mono)" }}>
 <span style={{ color: "var(--text-info)", fontWeight: 600 }}>{k}</span>
 <span style={{ color: "var(--text-muted)" }}>=</span>
 <span style={{ flex: 1 }}>{v}</span>
 <button onClick={() => copyText(v)} style={{ ...S.btn(), fontSize: 10, padding: "2px 6px" }}>Copy</button>
 </div>
 ))}
 </div>
 </div>
 )}
 </div>
 );
}

// ── Main Panel ───────────────────────────────────────────────────────────────

export function UtilitiesPanel() {
 const [activeTool, setActiveTool] = useState<ToolId>("jwt");

 return (
 <div style={{ display: "flex", height: "100%", overflow: "hidden" }}>
 {/* Sidebar */}
 <div style={{
 width: 80, flexShrink: 0, borderRight: "1px solid var(--border-color)",
 background: "var(--bg-secondary)", display: "flex", flexDirection: "column",
 paddingTop: 8,
 }}>
 {TOOL_LIST.map(({ id, label, icon }) => (
 <button
 key={id}
 onClick={() => setActiveTool(id)}
 style={{
 padding: "10px 4px", fontSize: 10, fontWeight: 600,
 background: activeTool === id ? "color-mix(in srgb, var(--accent-blue) 15%, transparent)" : "transparent",
 border: "none",
 borderLeft: activeTool === id ? "3px solid var(--accent-color)" : "3px solid transparent",
 color: activeTool === id ? "var(--accent-color)" : "var(--text-muted)",
 cursor: "pointer", display: "flex", flexDirection: "column",
 alignItems: "center", gap: 3, width: "100%",
 }}
 >
 <span style={{ fontSize: 16, fontFamily: "var(--font-mono)" }}>{icon}</span>
 <span>{label}</span>
 </button>
 ))}
 </div>

 {/* Content */}
 <div style={{ flex: 1, overflow: "auto", padding: 14 }}>
 <div style={{ fontSize: 13, fontWeight: 700, marginBottom: 12, color: "var(--text-primary)" }}>
 {TOOL_LIST.find((t) => t.id === activeTool)?.icon}{" "}
 {TOOL_LIST.find((t) => t.id === activeTool)?.label}
 </div>
 {activeTool === "jwt" && <JwtTool />}
 {activeTool === "json" && <JsonTool />}
 {activeTool === "regex" && <RegexTool />}
 {activeTool === "time" && <TimestampTool />}
 {activeTool === "b64" && <Base64Tool />}
 {activeTool === "hash" && <HashTool />}
 {activeTool === "url" && <UrlTool />}
 </div>
 </div>
 );
}
