/**
 * JsonToolsPanel — JSON developer utilities.
 *
 * Format : prettify / minify / sort-keys / validate with error location.
 * TypeScript: generate TS interfaces from any JSON object.
 * YAML : convert JSON → YAML (no external library — pure JS).
 * Query : dot-path + bracket accessor with path suggestions.
 *
 * Pure TypeScript — no Tauri commands required.
 */
import { useState, useMemo, useCallback } from "react";

// ── Helpers ────────────────────────────────────────────────────────────────────

function tryParse(text: string): { value: unknown; error: string | null } {
 try { return { value: JSON.parse(text), error: null }; }
 catch (e) { return { value: null, error: (e as Error).message }; }
}

function sortKeys(obj: unknown): unknown {
 if (Array.isArray(obj)) return obj.map(sortKeys);
 if (obj !== null && typeof obj === "object") {
 const sorted: Record<string, unknown> = {};
 Object.keys(obj as object).sort().forEach(k => {
 sorted[k] = sortKeys((obj as Record<string, unknown>)[k]);
 });
 return sorted;
 }
 return obj;
}

// ── JSON → YAML ────────────────────────────────────────────────────────────────

function toYaml(val: unknown, indent = 0): string {
 const pad = " ".repeat(indent);
 if (val === null) return "null";
 if (val === undefined) return "~";
 if (typeof val === "boolean" || typeof val === "number") return String(val);
 if (typeof val === "string") {
 // Needs quoting?
 if (/[:#\[\]{},|>&*!'"?@`]/.test(val) || val.includes("\n") || val.trim() !== val) {
 return JSON.stringify(val);
 }
 return val || '""';
 }
 if (Array.isArray(val)) {
 if (val.length === 0) return "[]";
 return val.map(item => {
 const rendered = toYaml(item, indent + 1);
 const firstLine = rendered.split("\n")[0];
 if (typeof item === "object" && item !== null) {
 return `${pad}- \n${rendered.split("\n").map(l => pad + " " + l).join("\n")}`.trimEnd();
 }
 return `${pad}- ${firstLine}`;
 }).join("\n");
 }
 if (typeof val === "object") {
 const entries = Object.entries(val as object);
 if (entries.length === 0) return "{}";
 return entries.map(([k, v]) => {
 const keyStr = /[^a-zA-Z0-9_-]/.test(k) ? JSON.stringify(k) : k;
 if (typeof v === "object" && v !== null && !Array.isArray(v) && Object.keys(v).length > 0) {
 return `${pad}${keyStr}:\n${toYaml(v, indent + 1)}`;
 }
 if (Array.isArray(v) && v.length > 0) {
 return `${pad}${keyStr}:\n${toYaml(v, indent + 1)}`;
 }
 return `${pad}${keyStr}: ${toYaml(v, indent + 1)}`;
 }).join("\n");
 }
 return String(val);
}

// ── JSON → TypeScript interfaces ───────────────────────────────────────────────

function tsType(val: unknown, depth = 0): string {
 if (val === null) return "null";
 if (Array.isArray(val)) {
 if (val.length === 0) return "unknown[]";
 const inner = tsType(val[0], depth);
 return `${inner}[]`;
 }
 switch (typeof val) {
 case "boolean": return "boolean";
 case "number": return "number";
 case "string": return "string";
 case "object": {
 if (depth > 5) return "Record<string, unknown>";
 const entries = Object.entries(val as object);
 if (entries.length === 0) return "Record<string, never>";
 const fields = entries.map(([k, v]) => {
 const key = /[^a-zA-Z0-9_$]/.test(k) ? `"${k}"` : k;
 return ` ${" ".repeat(depth)}${key}: ${tsType(v, depth + 1)};`;
 }).join("\n");
 return `{\n${fields}\n${" ".repeat(depth)}}`;
 }
 default: return "unknown";
 }
}

function generateInterfaces(root: unknown, rootName = "Root"): string {
 interface InterfaceDef { name: string; fields: { key: string; type: string }[] }
 const interfaces: InterfaceDef[] = [];

 function visit(val: unknown, name: string): string {
 if (val === null) return "null";
 if (Array.isArray(val)) {
 if (val.length === 0) return "unknown[]";
 return `${visit(val[0], name.replace(/s$/, "") || name + "Item")}[]`;
 }
 if (typeof val === "object") {
 const entries = Object.entries(val as object);
 const ifaceName = name.charAt(0).toUpperCase() + name.slice(1);
 const fields = entries.map(([k, v]) => {
 const childName = k.charAt(0).toUpperCase() + k.slice(1);
 return { key: k, type: visit(v, childName) };
 });
 interfaces.push({ name: ifaceName, fields });
 return ifaceName;
 }
 return tsType(val);
 }

 visit(root, rootName);
 // Deduplicate by name (last wins)
 const seen = new Map<string, InterfaceDef>();
 for (const i of interfaces) seen.set(i.name, i);
 return [...seen.values()].reverse().map(iface =>
 `export interface ${iface.name} {\n${iface.fields.map(f => {
 const key = /[^a-zA-Z0-9_$]/.test(f.key) ? `"${f.key}"` : f.key;
 return ` ${key}: ${f.type};`;
 }).join("\n")}\n}`
 ).join("\n\n");
}

// ── Dot-path query ─────────────────────────────────────────────────────────────

function queryPath(obj: unknown, path: string): { result: unknown; error: string | null } {
 if (!path.trim()) return { result: obj, error: null };
 try {
 // Tokenise: split on . and [] brackets
 const tokens = path.trim().split(/\.(?![^[]*\])|(?<=\])\.?|(?=\[)/).filter(Boolean);
 let cur = obj;
 for (const tok of tokens) {
 if (tok.startsWith("[") && tok.endsWith("]")) {
 const idx = tok.slice(1, -1).replace(/["']/g, "");
 cur = (cur as Record<string, unknown>)[idx];
 } else {
 cur = (cur as Record<string, unknown>)[tok];
 }
 if (cur === undefined) return { result: undefined, error: `Path not found at "${tok}"` };
 }
 return { result: cur, error: null };
 } catch (e) {
 return { result: null, error: (e as Error).message };
 }
}

function suggestPaths(obj: unknown, prefix = ""): string[] {
 if (typeof obj !== "object" || obj === null) return [];
 const results: string[] = [];
 const entries = Array.isArray(obj)
 ? obj.slice(0, 5).map((v, i) => [`[${i}]`, v] as [string, unknown])
 : Object.entries(obj as object).map(([k, v]) => [/[^a-zA-Z0-9_$]/.test(k) ? `["${k}"]` : `.${k}`, v] as [string, unknown]);
 for (const [seg, val] of entries) {
 const full = prefix + seg;
 results.push(full.startsWith(".") ? full.slice(1) : full);
 if (typeof val === "object" && val !== null && results.length < 30) {
 results.push(...suggestPaths(val, full));
 }
 }
 return results.slice(0, 30);
}

// ── Sample ─────────────────────────────────────────────────────────────────────

const SAMPLE = `{
 "user": {
 "id": 42,
 "name": "Alice Smith",
 "email": "alice@example.com",
 "roles": ["admin", "editor"],
 "active": true,
 "address": {
 "city": "San Francisco",
 "country": "US"
 }
 },
 "meta": {
 "version": "1.2.3",
 "createdAt": "2025-01-01T00:00:00Z"
 }
}`;

// ── Component ──────────────────────────────────────────────────────────────────

type SubTab = "format" | "typescript" | "yaml" | "query";

const INDENT_OPTIONS = [2, 4] as const;

export function JsonToolsPanel() {
 const [subTab, setSubTab] = useState<SubTab>("format");
 const [input, setInput] = useState(SAMPLE);
 const [indent, setIndent] = useState<2 | 4>(2);
 const [queryPath_, setQueryPath] = useState("user.address.city");
 const [copied, setCopied] = useState<string | null>(null);

 const { value: parsed, error: parseError } = useMemo(() => tryParse(input), [input]);

 // ── Format actions ──────────────────────────────────────────────────────────

 const prettify = () => { if (parsed !== null) setInput(JSON.stringify(parsed, null, indent)); };
 const minify = () => { if (parsed !== null) setInput(JSON.stringify(parsed)); };
 const sortAndFmt = () => { if (parsed !== null) setInput(JSON.stringify(sortKeys(parsed), null, indent)); };

 // ── Derived outputs ─────────────────────────────────────────────────────────

 const yamlOutput = useMemo(() => {
 if (!parsed) return "";
 try { return toYaml(parsed); } catch { return ""; }
 }, [parsed]);

 const tsOutput = useMemo(() => {
 if (!parsed || typeof parsed !== "object" || parsed === null) return "";
 try { return generateInterfaces(parsed); } catch { return ""; }
 }, [parsed]);

 const queryResult = useMemo(() => {
 if (!parsed) return { result: undefined, error: parseError };
 return queryPath(parsed, queryPath_);
 }, [parsed, queryPath_, parseError]);

 const suggestions = useMemo(() => parsed ? suggestPaths(parsed) : [], [parsed]);

 // ── Copy ────────────────────────────────────────────────────────────────────

 const copy = useCallback((text: string, key: string) => {
 navigator.clipboard.writeText(text);
 setCopied(key);
 setTimeout(() => setCopied(null), 1500);
 }, []);

 // ── Shared styles ───────────────────────────────────────────────────────────

 const outputStyle: React.CSSProperties = {
 margin: 0, padding: "10px 12px", fontFamily: "monospace", fontSize: 12,
 lineHeight: 1.7, whiteSpace: "pre-wrap", wordBreak: "break-all",
 background: "var(--bg-primary)", color: "var(--text-primary)",
 flex: 1, overflow: "auto",
 };

 return (
 <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>

 {/* Header */}
 <div style={{ padding: "8px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
 <span style={{ fontSize: 13, fontWeight: 600 }}>JSON Tools</span>
 <div style={{ display: "flex", gap: 4 }}>
 {(["format", "typescript", "yaml", "query"] as SubTab[]).map(t => (
 <button key={t} onClick={() => setSubTab(t)} style={{ padding: "2px 10px", fontSize: 10, borderRadius: 10, background: subTab === t ? "rgba(99,102,241,0.2)" : "var(--bg-primary)", border: `1px solid ${subTab === t ? "#6366f1" : "var(--border-color)"}`, color: subTab === t ? "#89b4fa" : "var(--text-muted)", cursor: "pointer", fontWeight: subTab === t ? 700 : 400 }}>
 {t === "format" ? "Format" : t === "typescript" ? "TypeScript" : t === "yaml" ? "YAML" : "Query"}
 </button>
 ))}
 </div>
 <div style={{ marginLeft: "auto", display: "flex", gap: 6, alignItems: "center" }}>
 {parseError
 ? <span style={{ fontSize: 10, color: "#f38ba8" }}>Invalid JSON</span>
 : <span style={{ fontSize: 10, color: "#a6e3a1" }}>✓ Valid JSON</span>
 }
 </div>
 </div>

 {/* Input area (always visible) */}
 <div style={{ display: "flex", flexDirection: "column", borderBottom: "1px solid var(--border-color)" }}>
 <div style={{ padding: "4px 12px", fontSize: 10, fontWeight: 700, color: "var(--text-muted)", background: "var(--bg-secondary)", display: "flex", gap: 8, alignItems: "center" }}>
 <span>INPUT</span>
 <button onClick={() => setInput(SAMPLE)} style={{ fontSize: 9, background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer" }}>Load sample</button>
 <button onClick={() => { navigator.clipboard.readText().then(t => setInput(t)).catch(() => {}); }} style={{ fontSize: 9, background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer" }}>Paste</button>
 <button onClick={() => setInput("")} style={{ fontSize: 9, background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer" }}>✕ Clear</button>
 {/* Indent selector */}
 <div style={{ marginLeft: "auto", display: "flex", gap: 4, alignItems: "center" }}>
 <span style={{ fontSize: 9, color: "var(--text-muted)" }}>indent:</span>
 {INDENT_OPTIONS.map(n => (
 <button key={n} onClick={() => setIndent(n)} style={{ fontSize: 9, padding: "1px 6px", borderRadius: 4, background: indent === n ? "rgba(99,102,241,0.2)" : "var(--bg-primary)", border: `1px solid ${indent === n ? "#6366f1" : "var(--border-color)"}`, color: indent === n ? "#89b4fa" : "var(--text-muted)", cursor: "pointer" }}>{n}</button>
 ))}
 </div>
 </div>
 <textarea value={input} onChange={e => setInput(e.target.value)} rows={7} spellCheck={false}
 style={{ resize: "vertical", padding: "8px 12px", fontSize: 12, fontFamily: "monospace", lineHeight: 1.6, background: parseError && input.trim() ? "rgba(243,139,168,0.04)" : "var(--bg-primary)", color: "var(--text-primary)", border: "none", outline: "none", width: "100%", boxSizing: "border-box" }} />
 {parseError && input.trim() && <div style={{ padding: "3px 12px", fontSize: 10, color: "#f38ba8", background: "rgba(243,139,168,0.06)" }}>{parseError}</div>}
 </div>

 {/* ── FORMAT TAB ── */}
 {subTab === "format" && (
 <div style={{ display: "flex", flexDirection: "column", flex: 1, overflow: "hidden" }}>
 <div style={{ padding: "6px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 6 }}>
 <button onClick={prettify} disabled={!parsed} style={{ padding: "3px 12px", fontSize: 11, background: "rgba(166,227,161,0.1)", border: "1px solid #a6e3a1", borderRadius: 4, color: "#a6e3a1", cursor: "pointer" }}>✦ Prettify</button>
 <button onClick={minify} disabled={!parsed} style={{ padding: "3px 12px", fontSize: 11, background: "rgba(250,179,135,0.1)", border: "1px solid #fab387", borderRadius: 4, color: "#fab387", cursor: "pointer" }}>⇲ Minify</button>
 <button onClick={sortAndFmt} disabled={!parsed} style={{ padding: "3px 12px", fontSize: 11, background: "rgba(137,180,250,0.1)", border: "1px solid #89b4fa", borderRadius: 4, color: "#89b4fa", cursor: "pointer" }}>⇅ Sort Keys</button>
 <div style={{ marginLeft: "auto" }}>
 <button onClick={() => copy(input, "fmt")} style={{ padding: "3px 10px", fontSize: 10, background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-muted)", cursor: "pointer" }}>
 {copied === "fmt" ? "✓ Copied" : "Copy"}
 </button>
 </div>
 </div>
 {parsed != null && (
 <div style={{ flex: 1, overflow: "auto", padding: "10px 12px", fontFamily: "monospace", fontSize: 12, lineHeight: 1.7, background: "var(--bg-primary)" }}>
 {/* Stats */}
 <div style={{ marginBottom: 8, display: "flex", gap: 8, flexWrap: "wrap" }}>
 {[
 ["type", Array.isArray(parsed) ? "array" : typeof parsed],
 ["size", `${input.length} chars`],
 ...(typeof parsed === "object" && parsed !== null
 ? [["keys", String(Object.keys(parsed).length)]]
 : []),
 ...(Array.isArray(parsed) ? [["items", String(parsed.length)]] : []),
 ].map(([k, v]) => (
 <span key={String(k)} style={{ fontSize: 10, padding: "1px 8px", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 10, color: "var(--text-muted)" }}>
 <span style={{ color: "#89b4fa" }}>{String(k)}</span>: {String(v)}
 </span>
 ))}
 </div>
 <pre style={{ margin: 0, whiteSpace: "pre-wrap", wordBreak: "break-all", color: "var(--text-primary)" }}>
 {JSON.stringify(parsed, null, indent)}
 </pre>
 </div>
 )}
 </div>
 )}

 {/* ── TYPESCRIPT TAB ── */}
 {subTab === "typescript" && (
 <div style={{ display: "flex", flexDirection: "column", flex: 1, overflow: "hidden" }}>
 <div style={{ padding: "4px 12px", fontSize: 10, fontWeight: 700, color: "#89b4fa", background: "var(--bg-secondary)", display: "flex", justifyContent: "space-between", alignItems: "center", borderBottom: "1px solid var(--border-color)" }}>
 <span>GENERATED INTERFACES</span>
 <button onClick={() => copy(tsOutput, "ts")} style={{ fontSize: 9, padding: "2px 8px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-muted)", cursor: "pointer" }}>{copied === "ts" ? "✓ Copied" : "Copy"}</button>
 </div>
 {!parsed
 ? <div style={{ padding: 16, color: "var(--text-muted)", fontSize: 12 }}>Fix JSON errors above to generate TypeScript interfaces.</div>
 : typeof parsed !== "object" || parsed === null
 ? <div style={{ padding: 16, color: "var(--text-muted)", fontSize: 12 }}>Paste a JSON object or array to generate interfaces.</div>
 : <pre style={outputStyle}>{tsOutput}</pre>
 }
 </div>
 )}

 {/* ── YAML TAB ── */}
 {subTab === "yaml" && (
 <div style={{ display: "flex", flexDirection: "column", flex: 1, overflow: "hidden" }}>
 <div style={{ padding: "4px 12px", fontSize: 10, fontWeight: 700, color: "#a6e3a1", background: "var(--bg-secondary)", display: "flex", justifyContent: "space-between", alignItems: "center", borderBottom: "1px solid var(--border-color)" }}>
 <span>YAML OUTPUT</span>
 <button onClick={() => copy(yamlOutput, "yaml")} style={{ fontSize: 9, padding: "2px 8px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-muted)", cursor: "pointer" }}>{copied === "yaml" ? "✓ Copied" : "Copy"}</button>
 </div>
 {!parsed
 ? <div style={{ padding: 16, color: "var(--text-muted)", fontSize: 12 }}>Fix JSON errors above to convert to YAML.</div>
 : <pre style={outputStyle}>{yamlOutput}</pre>
 }
 </div>
 )}

 {/* ── QUERY TAB ── */}
 {subTab === "query" && (
 <div style={{ display: "flex", flexDirection: "column", flex: 1, overflow: "hidden" }}>
 {/* Path input */}
 <div style={{ padding: "6px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 8, alignItems: "center" }}>
 <span style={{ fontSize: 11, color: "var(--text-muted)", flexShrink: 0 }}>Path:</span>
 <input value={queryPath_} onChange={e => setQueryPath(e.target.value)} placeholder="user.address.city or [0].name"
 style={{ flex: 1, padding: "4px 8px", fontSize: 12, fontFamily: "monospace", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none" }} />
 <button onClick={() => setQueryPath("")} style={{ fontSize: 10, background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer" }}>✕</button>
 </div>
 {/* Suggestions */}
 <div style={{ padding: "4px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 4, flexWrap: "wrap" }}>
 {suggestions.slice(0, 12).map(s => (
 <button key={s} onClick={() => setQueryPath(s)} style={{ fontSize: 9, fontFamily: "monospace", padding: "1px 6px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: queryPath_ === s ? "#89b4fa" : "var(--text-muted)", cursor: "pointer" }}>{s}</button>
 ))}
 </div>
 {/* Result */}
 <div style={{ flex: 1, overflow: "auto" }}>
 <div style={{ padding: "4px 12px", fontSize: 10, fontWeight: 700, color: queryResult.error ? "#f38ba8" : "#fab387", background: "var(--bg-secondary)", display: "flex", justifyContent: "space-between", alignItems: "center", borderBottom: "1px solid var(--border-color)" }}>
 <span>RESULT</span>
 {queryResult.result !== undefined && !queryResult.error && (
 <button onClick={() => copy(JSON.stringify(queryResult.result, null, indent), "qr")} style={{ fontSize: 9, padding: "2px 8px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-muted)", cursor: "pointer" }}>{copied === "qr" ? "✓" : ""}</button>
 )}
 </div>
 {queryResult.error
 ? <div style={{ padding: "10px 12px", fontSize: 12, color: "#f38ba8" }}>{queryResult.error}</div>
 : queryResult.result === undefined
 ? <div style={{ padding: "10px 12px", fontSize: 12, color: "var(--text-muted)", fontStyle: "italic" }}>undefined</div>
 : <pre style={outputStyle}>{JSON.stringify(queryResult.result, null, indent)}</pre>
 }
 </div>
 </div>
 )}

 </div>
 );
}
