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
import { Minimize2, ArrowUpDown, Wand2 } from "lucide-react";

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
 if (/[:#[\]{},|>&*!'"?@`]/.test(val) || val.includes("\n") || val.trim() !== val) {
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
 margin: 0, padding: "10px 12px", fontFamily: "var(--font-mono)", fontSize: "var(--font-size-base)",
 lineHeight: 1.7, whiteSpace: "pre-wrap", wordBreak: "break-all",
 background: "var(--bg-primary)", color: "var(--text-primary)",
 flex: 1, overflow: "auto",
 };

 return (
 <div className="panel-container">

 {/* Header */}
 <div className="panel-header" style={{ flexWrap: "wrap" }}>
 <span style={{ fontSize: "var(--font-size-md)", fontWeight: 600 }}>JSON Tools</span>
 <div style={{ display: "flex", gap: 4 }}>
 {(["format", "typescript", "yaml", "query"] as SubTab[]).map(t => (
 <button key={t} onClick={() => setSubTab(t)} className={`panel-tab ${subTab === t ? "active" : ""}`}>
 {t === "format" ? "Format" : t === "typescript" ? "TypeScript" : t === "yaml" ? "YAML" : "Query"}
 </button>
 ))}
 </div>
 <div style={{ marginLeft: "auto", display: "flex", gap: 6, alignItems: "center" }}>
 {parseError
 ? <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-danger)" }}>Invalid JSON</span>
 : <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-success)" }}>✓ Valid JSON</span>
 }
 </div>
 </div>

 {/* Input area (always visible) */}
 <div style={{ display: "flex", flexDirection: "column", borderBottom: "1px solid var(--border-color)" }}>
 <div style={{ padding: "4px 12px", fontSize: "var(--font-size-xs)", fontWeight: 700, color: "var(--text-secondary)", background: "var(--bg-secondary)", display: "flex", gap: 8, alignItems: "center" }}>
 <span>INPUT</span>
 <button onClick={() => setInput(SAMPLE)} style={{ fontSize: 9, background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer" }}>Load sample</button>
 <button onClick={() => { navigator.clipboard.readText().then(t => setInput(t)).catch(() => {}); }} style={{ fontSize: 9, background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer" }}>Paste</button>
 <button onClick={() => setInput("")} style={{ fontSize: 9, background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer" }}>✕ Clear</button>
 {/* Indent selector */}
 <div style={{ marginLeft: "auto", display: "flex", gap: 4, alignItems: "center" }}>
 <span style={{ fontSize: 9, color: "var(--text-secondary)" }}>indent:</span>
 {INDENT_OPTIONS.map(n => (
 <button key={n} onClick={() => setIndent(n)} style={{ fontSize: 9, padding: "1px 8px", borderRadius: "var(--radius-xs-plus)", background: indent === n ? "color-mix(in srgb, var(--accent-blue) 20%, transparent)" : "var(--bg-primary)", border: `1px solid ${indent === n ? "var(--accent-color)" : "var(--border-color)"}`, color: indent === n ? "var(--accent-color)" : "var(--text-secondary)", cursor: "pointer" }}>{n}</button>
 ))}
 </div>
 </div>
 <textarea value={input} onChange={e => setInput(e.target.value)} rows={7} spellCheck={false}
 style={{ resize: "vertical", padding: "8px 12px", fontSize: "var(--font-size-base)", fontFamily: "var(--font-mono)", lineHeight: 1.6, background: parseError && input.trim() ? "rgba(243,139,168,0.04)" : "var(--bg-primary)", color: "var(--text-primary)", border: "none", outline: "none", width: "100%", boxSizing: "border-box" }} />
 {parseError && input.trim() && <div style={{ padding: "3px 12px", fontSize: "var(--font-size-xs)", color: "var(--text-danger)", background: "color-mix(in srgb, var(--accent-rose) 6%, transparent)" }}>{parseError}</div>}
 </div>

 {/* ── FORMAT TAB ── */}
 {subTab === "format" && (
 <div style={{ display: "flex", flexDirection: "column", flex: 1, overflow: "hidden" }}>
 <div style={{ padding: "8px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 6 }}>
 <button className="panel-btn" onClick={prettify} disabled={!parsed} style={{ padding: "3px 12px", fontSize: "var(--font-size-sm)", background: "color-mix(in srgb, var(--accent-green) 10%, transparent)", border: "1px solid var(--success-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-success)", cursor: "pointer", display: "inline-flex", alignItems: "center", gap: 4 }}><Wand2 size={11} strokeWidth={1.5} /> Prettify</button>
 <button className="panel-btn" onClick={minify} disabled={!parsed} style={{ padding: "3px 12px", fontSize: "var(--font-size-sm)", background: "rgba(250,179,135,0.1)", border: "1px solid var(--warning-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-warning-alt)", cursor: "pointer", display: "inline-flex", alignItems: "center", gap: 4 }}><Minimize2 size={11} strokeWidth={1.5} /> Minify</button>
 <button className="panel-btn" onClick={sortAndFmt} disabled={!parsed} style={{ padding: "3px 12px", fontSize: "var(--font-size-sm)", background: "color-mix(in srgb, var(--accent-blue) 10%, transparent)", border: "1px solid var(--accent-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-info)", cursor: "pointer", display: "inline-flex", alignItems: "center", gap: 4 }}><ArrowUpDown size={11} strokeWidth={1.5} /> Sort Keys</button>
 <div style={{ marginLeft: "auto" }}>
 <button onClick={() => copy(input, "fmt")} style={{ padding: "3px 12px", fontSize: "var(--font-size-xs)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-secondary)", cursor: "pointer" }}>
 {copied === "fmt" ? "✓ Copied" : "Copy"}
 </button>
 </div>
 </div>
 {parsed != null && (
 <div style={{ flex: 1, overflow: "auto", padding: "12px 12px", fontFamily: "var(--font-mono)", fontSize: "var(--font-size-base)", lineHeight: 1.7, background: "var(--bg-primary)" }}>
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
 <span key={String(k)} style={{ fontSize: "var(--font-size-xs)", padding: "1px 8px", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-md)", color: "var(--text-secondary)" }}>
 <span style={{ color: "var(--text-info)" }}>{String(k)}</span>: {String(v)}
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
 <div style={{ padding: "4px 12px", fontSize: "var(--font-size-xs)", fontWeight: 700, color: "var(--text-info)", background: "var(--bg-secondary)", display: "flex", justifyContent: "space-between", alignItems: "center", borderBottom: "1px solid var(--border-color)" }}>
 <span>GENERATED INTERFACES</span>
 <button onClick={() => copy(tsOutput, "ts")} style={{ fontSize: 9, padding: "2px 8px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-secondary)", cursor: "pointer" }}>{copied === "ts" ? "✓ Copied" : "Copy"}</button>
 </div>
 {!parsed
 ? <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: "var(--font-size-base)" }}>Fix JSON errors above to generate TypeScript interfaces.</div>
 : typeof parsed !== "object" || parsed === null
 ? <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: "var(--font-size-base)" }}>Paste a JSON object or array to generate interfaces.</div>
 : <pre style={outputStyle}>{tsOutput}</pre>
 }
 </div>
 )}

 {/* ── YAML TAB ── */}
 {subTab === "yaml" && (
 <div style={{ display: "flex", flexDirection: "column", flex: 1, overflow: "hidden" }}>
 <div style={{ padding: "4px 12px", fontSize: "var(--font-size-xs)", fontWeight: 700, color: "var(--text-success)", background: "var(--bg-secondary)", display: "flex", justifyContent: "space-between", alignItems: "center", borderBottom: "1px solid var(--border-color)" }}>
 <span>YAML OUTPUT</span>
 <button onClick={() => copy(yamlOutput, "yaml")} style={{ fontSize: 9, padding: "2px 8px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-secondary)", cursor: "pointer" }}>{copied === "yaml" ? "✓ Copied" : "Copy"}</button>
 </div>
 {!parsed
 ? <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: "var(--font-size-base)" }}>Fix JSON errors above to convert to YAML.</div>
 : <pre style={outputStyle}>{yamlOutput}</pre>
 }
 </div>
 )}

 {/* ── QUERY TAB ── */}
 {subTab === "query" && (
 <div style={{ display: "flex", flexDirection: "column", flex: 1, overflow: "hidden" }}>
 {/* Path input */}
 <div style={{ padding: "8px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 8, alignItems: "center" }}>
 <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", flexShrink: 0 }}>Path:</span>
 <input value={queryPath_} onChange={e => setQueryPath(e.target.value)} placeholder="user.address.city or [0].name"
 style={{ flex: 1, padding: "4px 8px", fontSize: "var(--font-size-base)", fontFamily: "var(--font-mono)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", outline: "none" }} />
 <button onClick={() => setQueryPath("")} style={{ fontSize: "var(--font-size-xs)", background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer" }}>✕</button>
 </div>
 {/* Suggestions */}
 <div style={{ padding: "4px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 4, flexWrap: "wrap" }}>
 {suggestions.slice(0, 12).map(s => (
 <button key={s} onClick={() => setQueryPath(s)} style={{ fontSize: 9, fontFamily: "var(--font-mono)", padding: "1px 8px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: queryPath_ === s ? "var(--accent-color)" : "var(--text-secondary)", cursor: "pointer" }}>{s}</button>
 ))}
 </div>
 {/* Result */}
 <div style={{ flex: 1, overflow: "auto" }}>
 <div style={{ padding: "4px 12px", fontSize: "var(--font-size-xs)", fontWeight: 700, color: queryResult.error ? "var(--error-color)" : "var(--warning-color)", background: "var(--bg-secondary)", display: "flex", justifyContent: "space-between", alignItems: "center", borderBottom: "1px solid var(--border-color)" }}>
 <span>RESULT</span>
 {queryResult.result !== undefined && !queryResult.error && (
 <button onClick={() => copy(JSON.stringify(queryResult.result, null, indent), "qr")} style={{ fontSize: 9, padding: "2px 8px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-secondary)", cursor: "pointer" }}>{copied === "qr" ? "✓" : ""}</button>
 )}
 </div>
 {queryResult.error
 ? <div style={{ padding: "12px 12px", fontSize: "var(--font-size-base)", color: "var(--text-danger)" }}>{queryResult.error}</div>
 : queryResult.result === undefined
 ? <div style={{ padding: "12px 12px", fontSize: "var(--font-size-base)", color: "var(--text-secondary)", fontStyle: "italic" }}>undefined</div>
 : <pre style={outputStyle}>{JSON.stringify(queryResult.result, null, indent)}</pre>
 }
 </div>
 </div>
 )}

 </div>
 );
}
