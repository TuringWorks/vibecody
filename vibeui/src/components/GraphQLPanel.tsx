/**
 * GraphQLPanel — GraphQL Playground.
 *
 * Schema introspection, query/mutation editor with variables, results viewer,
 * custom headers, and query history. Complements the existing HTTP Playground
 * and API Docs viewer.
 */
import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

interface GraphQLResult {
 data: unknown;
 errors: unknown;
 status: number;
 duration_ms: number;
 raw: string;
}

interface GraphQLSchemaField {
 name: string;
 kind: string;
 description: string | null;
}

interface GraphQLType {
 name: string;
 kind: string;
 description: string | null;
 fields: GraphQLSchemaField[];
}

interface GraphQLSchema {
 query_type: string | null;
 mutation_type: string | null;
 subscription_type: string | null;
 types: GraphQLType[];
}

interface HistoryEntry {
 url: string;
 query: string;
 variables: string;
 timestamp: number;
 duration_ms: number;
 success: boolean;
}

const KIND_COLORS: Record<string, string> = {
 OBJECT: "#89b4fa",
 INPUT_OBJECT: "#cba6f7",
 ENUM: "#a6e3a1",
 INTERFACE: "#89dceb",
 UNION: "#f9e2af",
 SCALAR: "#fab387",
};

const EXAMPLE_QUERIES: Record<string, string> = {
 "GitHub API": `{
 viewer {
 login
 name
 repositories(first: 5) {
 nodes { name stargazerCount }
 }
 }
}`,
 "Countries API": `{
 countries(filter: { continent: { eq: "EU" } }) {
 name capital currency
 }
}`,
 "SpaceX API": `{
 launches(limit: 3, sort: "launch_date_utc", order: "desc") {
 mission_name launch_date_utc
 rocket { rocket_name }
 }
}`,
};

const PRESET_URLS = [
 "https://countries.trevorblades.com/graphql",
 "https://api.spacex.land/graphql/",
 "https://api.github.com/graphql",
];

export function GraphQLPanel() {
 const [url, setUrl] = useState("https://countries.trevorblades.com/graphql");
 const [query, setQuery] = useState(`{
 countries(filter: { continent: { eq: "EU" } }) {
 name capital currency
 }
}`);
 const [variables, setVariables] = useState("{}");
 const [headersText, setHeadersText] = useState("{}");
 const [operationName, setOperationName] = useState("");
 const [result, setResult] = useState<GraphQLResult | null>(null);
 const [schema, setSchema] = useState<GraphQLSchema | null>(null);
 const [running, setRunning] = useState(false);
 const [introspecting, setIntrospecting] = useState(false);
 const [error, setError] = useState<string | null>(null);
 const [tab, setTab] = useState<"query" | "schema" | "history">("query");
 const [viewTab, setViewTab] = useState<"result" | "raw">("result");
 const [history, setHistory] = useState<HistoryEntry[]>([]);
 const [schemaSearch, setSchemaSearch] = useState("");
 const [expandedType, setExpandedType] = useState<string | null>(null);
 const queryRef = useRef<HTMLTextAreaElement>(null);

 // Load history from localStorage
 useEffect(() => {
 try {
 const saved = localStorage.getItem("vibe-gql-history");
 if (saved) setHistory(JSON.parse(saved));
 } catch { /* ignore */ }
 }, []);

 const saveHistory = (entries: HistoryEntry[]) => {
 setHistory(entries);
 try { localStorage.setItem("vibe-gql-history", JSON.stringify(entries.slice(0, 50))); } catch { /* ignore */ }
 };

 const parseHeaders = (): Record<string, string> | undefined => {
 try {
 const h = JSON.parse(headersText);
 if (typeof h === "object" && h !== null) return h as Record<string, string>;
 } catch { /* ignore */ }
 return undefined;
 };

 const parseVariables = (): unknown | undefined => {
 try {
 const v = JSON.parse(variables.trim() || "{}");
 return Object.keys(v).length ? v : undefined;
 } catch { return undefined; }
 };

 const runQuery = async () => {
 if (!url || !query.trim() || running) return;
 setRunning(true);
 setError(null);
 try {
 const res = await invoke<GraphQLResult>("run_graphql_query", {
 url,
 query: query.trim(),
 variables: parseVariables(),
 headers: parseHeaders(),
 operationName: operationName.trim() || null,
 });
 setResult(res);

 // Save to history
 const entry: HistoryEntry = {
 url, query: query.trim(), variables,
 timestamp: Date.now(),
 duration_ms: res.duration_ms,
 success: !res.errors,
 };
 saveHistory([entry, ...history.filter((h) => h.query !== query.trim() || h.url !== url)]);
 } catch (e) {
 setError(String(e));
 } finally {
 setRunning(false);
 }
 };

 const introspect = async () => {
 if (!url || introspecting) return;
 setIntrospecting(true);
 setError(null);
 try {
 const s = await invoke<GraphQLSchema>("introspect_graphql_schema", {
 url,
 headers: parseHeaders(),
 });
 setSchema(s);
 setTab("schema");
 } catch (e) {
 setError(String(e));
 } finally {
 setIntrospecting(false);
 }
 };

 const loadFromHistory = (entry: HistoryEntry) => {
 setUrl(entry.url);
 setQuery(entry.query);
 setVariables(entry.variables);
 setTab("query");
 };

 const filteredTypes = schema?.types.filter((t) =>
 !schemaSearch || t.name.toLowerCase().includes(schemaSearch.toLowerCase())
 ) ?? [];

 const TAB = (id: string, label: string, active: boolean) => (
 <button
 key={id}
 onClick={() => setTab(id as typeof tab)}
 style={{
 padding: "6px 14px", fontSize: 11, fontWeight: active ? 600 : 400,
 background: active ? "rgba(99,102,241,0.15)" : "transparent",
 color: active ? "var(--accent-color, #6366f1)" : "var(--text-muted)",
 border: "none", borderBottom: active ? "2px solid var(--accent-color, #6366f1)" : "2px solid transparent",
 cursor: "pointer",
 }}
 >
 {label}
 </button>
 );

 return (
 <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
 {/* Header */}
 <div style={{
 padding: "8px 12px", borderBottom: "1px solid var(--border-color)",
 background: "var(--bg-secondary)", flexShrink: 0,
 }}>
 <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
 <span style={{ fontSize: 15 }}></span>
 <input
 value={url}
 onChange={(e) => setUrl(e.target.value)}
 list="gql-urls"
 placeholder="https://api.example.com/graphql"
 style={{
 flex: 1, padding: "5px 9px", fontSize: 12, fontFamily: "monospace",
 background: "var(--bg-primary)", border: "1px solid var(--border-color)",
 borderRadius: 4, color: "var(--text-primary)", outline: "none",
 }}
 />
 <datalist id="gql-urls">
 {PRESET_URLS.map((u) => <option key={u} value={u} />)}
 </datalist>
 <button
 onClick={introspect}
 disabled={introspecting || !url}
 style={{
 padding: "5px 12px", fontSize: 11, cursor: "pointer",
 background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
 borderRadius: 4, color: "var(--text-secondary)",
 }}
 >
 {introspecting ? "" : "Schema"}
 </button>
 <button
 onClick={runQuery}
 disabled={running || !url || !query.trim()}
 style={{
 padding: "5px 14px", fontSize: 11, fontWeight: 600, cursor: "pointer",
 background: running ? "var(--bg-secondary)" : "var(--accent-color, #6366f1)",
 color: running ? "var(--text-muted)" : "var(--text-primary, #fff)",
 border: "none", borderRadius: 4,
 }}
 >
 {running ? "Running…" : "Run"}
 </button>
 </div>

 {/* Quick load examples */}
 <div style={{ display: "flex", gap: 5, marginTop: 6, flexWrap: "wrap" }}>
 {Object.entries(EXAMPLE_QUERIES).map(([label, q]) => (
 <button
 key={label}
 onClick={() => setQuery(q)}
 style={{
 padding: "2px 8px", fontSize: 10, borderRadius: 10,
 background: "var(--bg-primary)", border: "1px solid var(--border-color)",
 color: "var(--text-muted)", cursor: "pointer",
 }}
 >
 {label}
 </button>
 ))}
 </div>
 </div>

 {error && (
 <div style={{ margin: "6px 12px", padding: "6px 10px", background: "var(--error-bg, #2a1a1a)", color: "var(--text-danger, #f38ba8)", borderRadius: 4, fontSize: 11 }}>
 {error}
 </div>
 )}

 {/* Sub-tabs */}
 <div style={{ display: "flex", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", flexShrink: 0 }}>
 {TAB("query", "Query", tab === "query")}
 {TAB("schema", `Schema${schema ? ` (${schema.types.length})` : ""}`, tab === "schema")}
 {TAB("history", `History (${history.length})`, tab === "history")}
 </div>

 {/* ── Query tab ─────────────────────────────────────────────────── */}
 {tab === "query" && (
 <div style={{ flex: 1, overflow: "hidden", display: "flex", gap: 0 }}>
 {/* Left: editor pane */}
 <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden", borderRight: "1px solid var(--border-color)" }}>
 <div style={{ padding: "4px 10px", fontSize: 10, fontWeight: 600, color: "var(--text-muted)", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)" }}>
 QUERY / MUTATION
 </div>
 <textarea
 ref={queryRef}
 value={query}
 onChange={(e) => setQuery(e.target.value)}
 onKeyDown={(e) => { if ((e.metaKey || e.ctrlKey) && e.key === "Enter") { e.preventDefault(); runQuery(); } }}
 style={{
 flex: 1, padding: "10px", fontSize: 12, fontFamily: "monospace",
 background: "var(--bg-primary)", border: "none", color: "var(--text-primary)",
 outline: "none", resize: "none", lineHeight: 1.6,
 }}
 />

 {/* Variables */}
 <div style={{ borderTop: "1px solid var(--border-color)" }}>
 <div style={{ padding: "3px 10px", fontSize: 10, fontWeight: 600, color: "var(--text-muted)", background: "var(--bg-secondary)", display: "flex", gap: 8, alignItems: "center" }}>
 <span>VARIABLES</span>
 {operationName !== undefined && (
 <input
 value={operationName}
 onChange={(e) => setOperationName(e.target.value)}
 placeholder="operationName (optional)"
 style={{ marginLeft: "auto", padding: "2px 6px", fontSize: 10, background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 3, color: "var(--text-muted)", width: 180, outline: "none" }}
 />
 )}
 </div>
 <textarea
 value={variables}
 onChange={(e) => setVariables(e.target.value)}
 rows={3}
 style={{
 width: "100%", boxSizing: "border-box",
 padding: "6px 10px", fontSize: 11, fontFamily: "monospace",
 background: "var(--bg-secondary)", border: "none",
 color: "var(--text-muted)", outline: "none", resize: "none",
 }}
 />
 </div>

 {/* Headers */}
 <div style={{ borderTop: "1px solid var(--border-color)" }}>
 <div style={{ padding: "3px 10px", fontSize: 10, fontWeight: 600, color: "var(--text-muted)", background: "var(--bg-secondary)" }}>
 HEADERS (JSON)
 </div>
 <textarea
 value={headersText}
 onChange={(e) => setHeadersText(e.target.value)}
 rows={2}
 placeholder='{"Authorization": "Bearer TOKEN"}'
 style={{
 width: "100%", boxSizing: "border-box",
 padding: "6px 10px", fontSize: 11, fontFamily: "monospace",
 background: "var(--bg-secondary)", border: "none",
 color: "var(--text-muted)", outline: "none", resize: "none",
 }}
 />
 </div>
 </div>

 {/* Right: result pane */}
 <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
 <div style={{ display: "flex", alignItems: "center", padding: "4px 10px", background: "var(--bg-secondary)", borderBottom: "1px solid var(--border-color)", gap: 6 }}>
 <span style={{ fontSize: 10, fontWeight: 600, color: "var(--text-muted)", marginRight: 4 }}>RESPONSE</span>
 {result && (
 <>
 <span style={{
 fontSize: 10, fontWeight: 600, padding: "1px 6px", borderRadius: 10,
 background: result.errors ? "rgba(243,139,168,0.15)" : "rgba(166,227,161,0.15)",
 color: result.errors ? "var(--error-color, #f38ba8)" : "var(--success-color, #a6e3a1)",
 }}>
 {result.status} {result.errors ? "errors" : "OK"}
 </span>
 <span style={{ fontSize: 10, color: "var(--text-muted)" }}>{result.duration_ms}ms</span>
 </>
 )}
 <div style={{ marginLeft: "auto", display: "flex", gap: 4 }}>
 {(["result", "raw"] as const).map((v) => (
 <button
 key={v}
 onClick={() => setViewTab(v)}
 style={{
 padding: "2px 8px", fontSize: 10, borderRadius: 3,
 background: viewTab === v ? "var(--accent-color, #6366f1)" : "var(--bg-secondary)",
 color: viewTab === v ? "var(--text-primary, #fff)" : "var(--text-muted)",
 border: "1px solid var(--border-color)", cursor: "pointer",
 }}
 >
 {v}
 </button>
 ))}
 {result && <button onClick={() => navigator.clipboard.writeText(result.raw).catch(() => {})} style={{ padding: "2px 8px", fontSize: 10, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 3, color: "var(--text-muted)", cursor: "pointer" }}>Copy</button>}
 </div>
 </div>

 <div style={{ flex: 1, overflow: "auto", padding: 10, fontFamily: "monospace", fontSize: 11, lineHeight: 1.6, background: "var(--bg-primary, #0d1117)", color: "var(--text-primary, #e6edf3)", whiteSpace: "pre-wrap", wordBreak: "break-all" }}>
 {!result && !running && (
 <span style={{ color: "var(--text-secondary, #6b7280)" }}>
 Run a query to see results here.{"\n"}Tip: Cmd+Enter to run.
 </span>
 )}
 {running && <span style={{ color: "var(--info-color, #89dceb)" }}>Running…▌</span>}
 {result && viewTab === "raw" && result.raw}
 {result && viewTab === "result" && (() => {
 if (result.errors) {
 return (
 <div>
 <div style={{ color: "var(--text-danger, #f38ba8)", fontWeight: 600, marginBottom: 6 }}>Errors:</div>
 <pre style={{ margin: 0 }}>{JSON.stringify(result.errors, null, 2)}</pre>
 {Boolean(result.data) && (
 <>
 <div style={{ color: "var(--text-warning, #f9e2af)", fontWeight: 600, margin: "10px 0 6px" }}>Partial Data:</div>
 <pre style={{ margin: 0 }}>{JSON.stringify(result.data, null, 2)}</pre>
 </>
 )}
 </div>
 );
 }
 return <pre style={{ margin: 0 }}>{JSON.stringify(result.data, null, 2)}</pre>;
 })()}
 </div>
 </div>
 </div>
 )}

 {/* ── Schema tab ────────────────────────────────────────────────── */}
 {tab === "schema" && (
 <div style={{ flex: 1, overflow: "hidden", display: "flex", flexDirection: "column" }}>
 {!schema ? (
 <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center", color: "var(--text-muted)", fontSize: 12 }}>
 Click "Schema" to introspect the API.
 </div>
 ) : (
 <>
 {/* Root types */}
 <div style={{ padding: "8px 12px", background: "var(--bg-secondary)", borderBottom: "1px solid var(--border-color)", display: "flex", gap: 12, fontSize: 11 }}>
 {[["Query", schema.query_type], ["Mutation", schema.mutation_type], ["Subscription", schema.subscription_type]].map(([label, name]) => name && (
 <span key={label} style={{ display: "flex", gap: 5, alignItems: "center" }}>
 <span style={{ color: "var(--text-muted)", fontWeight: 600 }}>{label}:</span>
 <span style={{ fontFamily: "monospace", color: "var(--text-info, #89b4fa)" }}>{name}</span>
 </span>
 ))}
 <input
 value={schemaSearch}
 onChange={(e) => setSchemaSearch(e.target.value)}
 placeholder="Filter types…"
 style={{ marginLeft: "auto", padding: "2px 8px", fontSize: 11, background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 3, color: "var(--text-primary)", outline: "none", width: 150 }}
 />
 </div>

 {/* Type list */}
 <div style={{ flex: 1, overflow: "auto", padding: "6px 10px", display: "flex", flexDirection: "column", gap: 3 }}>
 {filteredTypes.map((t) => (
 <div key={t.name} style={{ borderRadius: 4, border: "1px solid var(--border-color)", overflow: "hidden" }}>
 <div
 onClick={() => setExpandedType(expandedType === t.name ? null : t.name)}
 style={{
 padding: "6px 10px", display: "flex", alignItems: "center", gap: 8,
 background: "var(--bg-secondary)", cursor: "pointer",
 }}
 >
 <span style={{ fontSize: 10, fontWeight: 700, padding: "1px 5px", borderRadius: 3, background: KIND_COLORS[t.kind] ? `${KIND_COLORS[t.kind]}22` : "transparent", color: KIND_COLORS[t.kind] ?? "var(--text-muted)" }}>
 {t.kind}
 </span>
 <span style={{ fontSize: 12, fontFamily: "monospace", fontWeight: 600 }}>{t.name}</span>
 {t.description && <span style={{ fontSize: 10, color: "var(--text-muted)", flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{t.description}</span>}
 <span style={{ fontSize: 10, color: "var(--text-muted)", marginLeft: "auto" }}>{t.fields.length} fields {expandedType === t.name ? "" : "▼"}</span>
 </div>
 {expandedType === t.name && t.fields.length > 0 && (
 <div style={{ padding: "4px 0", background: "var(--bg-primary)" }}>
 {t.fields.map((f) => (
 <div key={f.name} style={{ padding: "3px 20px", display: "flex", gap: 10, fontSize: 11, borderBottom: "1px solid var(--border-color)" }}>
 <span style={{ fontFamily: "monospace", color: "var(--text-info, #89b4fa)", width: 180, flexShrink: 0 }}>{f.name}</span>
 <span style={{ fontSize: 10, color: "var(--text-muted)", fontWeight: 600 }}>{f.kind}</span>
 {f.description && <span style={{ fontSize: 10, color: "var(--text-muted)" }}>{f.description}</span>}
 </div>
 ))}
 </div>
 )}
 </div>
 ))}
 </div>
 </>
 )}
 </div>
 )}

 {/* ── History tab ───────────────────────────────────────────────── */}
 {tab === "history" && (
 <div style={{ flex: 1, overflow: "auto", padding: "8px 12px", display: "flex", flexDirection: "column", gap: 5 }}>
 {history.length === 0 && (
 <div style={{ textAlign: "center", padding: "30px 0", color: "var(--text-muted)", fontSize: 12 }}>
 No query history yet. Run a query first.
 </div>
 )}
 {history.length > 0 && (
 <div style={{ display: "flex", justifyContent: "flex-end", marginBottom: 4 }}>
 <button onClick={() => saveHistory([])} style={{ fontSize: 10, background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer" }}>Clear all</button>
 </div>
 )}
 {history.map((entry, i) => (
 <div
 key={i}
 onClick={() => loadFromHistory(entry)}
 style={{
 padding: "8px 10px", borderRadius: 5, cursor: "pointer",
 background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
 }}
 >
 <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 3 }}>
 <span style={{ fontSize: 10, fontFamily: "monospace", color: "var(--text-info, #89b4fa)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", flex: 1 }}>{entry.url}</span>
 <span style={{ fontSize: 10, color: "var(--text-muted)", flexShrink: 0, marginLeft: 8 }}>{new Date(entry.timestamp).toLocaleTimeString()} · {entry.duration_ms}ms</span>
 <span style={{ marginLeft: 6, fontSize: 10, color: entry.success ? "var(--success-color, #a6e3a1)" : "var(--error-color, #f38ba8)" }}>{entry.success ? "" : ""}</span>
 </div>
 <pre style={{ margin: 0, fontSize: 10, color: "var(--text-muted)", overflow: "hidden", maxHeight: 40, fontFamily: "monospace", whiteSpace: "pre-wrap" }}>
 {entry.query.slice(0, 120)}{entry.query.length > 120 ? "…" : ""}
 </pre>
 </div>
 ))}
 </div>
 )}
 </div>
 );
}
