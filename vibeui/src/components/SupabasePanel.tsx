import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface SupabaseConfig {
 url: string;
 anon_key: string;
}

interface TableInfo {
 name: string;
 row_count: number;
}

interface QueryResult {
 columns: string[];
 rows: string[][];
 error: string | null;
}

export function SupabasePanel({ workspacePath, provider }: { workspacePath: string | null; provider: string }) {
 const [config, setConfig] = useState<SupabaseConfig>({ url: "", anon_key: "" });
 const [connected, setConnected] = useState(false);
 const [tables, setTables] = useState<TableInfo[]>([]);
 const [selectedTable, setSelectedTable] = useState<string | null>(null);
 const [queryResult, setQueryResult] = useState<QueryResult | null>(null);
 const [query, setQuery] = useState("");
 const [naturalQuery, setNaturalQuery] = useState("");
 const [loading, setLoading] = useState(false);
 const [error, setError] = useState<string | null>(null);
 const [activeTab, setActiveTab] = useState<"tables" | "query" | "ai">("tables");

 // Load saved config on mount (must be before early return to satisfy Rules of Hooks)
 useEffect(() => {
 if (!workspacePath) return;
 let cancelled = false;
 invoke<SupabaseConfig>("get_supabase_config", { workspacePath })
 .then(cfg => { if (!cancelled && cfg.url) { setConfig(cfg); setConnected(true); fetchTables(cfg); } })
 .catch((e: unknown) => { if (!cancelled) console.error("Failed to load Supabase config:", e); });
 return () => { cancelled = true; };
 }, [workspacePath]);

 if (!workspacePath) {
 return <div className="empty-state"><p>Open a workspace folder to use the Supabase panel.</p></div>;
 }

 const fetchTables = async (cfg?: SupabaseConfig) => {
 const c = cfg || config;
 if (!c.url || !c.anon_key) return;
 setLoading(true);
 try {
 const tbls = await invoke<TableInfo[]>("list_supabase_tables", { url: c.url, anonKey: c.anon_key });
 setTables(tbls);
 setConnected(true);
 setError(null);
 } catch (e) {
 setError(String(e));
 setConnected(false);
 } finally {
 setLoading(false);
 }
 };

 const saveConfig = async () => {
 setLoading(true);
 try {
 await invoke("save_supabase_config", { workspacePath, url: config.url, anonKey: config.anon_key });
 await fetchTables();
 } catch (e) {
 setError(String(e));
 } finally {
 setLoading(false);
 }
 };

 const runQuery = async () => {
 if (!query.trim()) return;
 setLoading(true);
 try {
 const result = await invoke<QueryResult>("query_supabase", {
 url: config.url, anonKey: config.anon_key, sql: query
 });
 setQueryResult(result);
 if (result.error) setError(result.error);
 } catch (e) {
 setError(String(e));
 } finally {
 setLoading(false);
 }
 };

 const generateQuery = async () => {
 if (!naturalQuery.trim()) return;
 setLoading(true);
 try {
 const sql = await invoke<string>("generate_supabase_query", {
 workspacePath, provider,
 description: naturalQuery,
 tables: tables.map(t => t.name)
 });
 setQuery(sql);
 setActiveTab("query");
 } catch (e) {
 setError(String(e));
 } finally {
 setLoading(false);
 }
 };

 const selectTable = async (table: string) => {
 setSelectedTable(table);
 setQuery(`SELECT * FROM "${table}"LIMIT 50`);
 setActiveTab("query");
 };

 const s = {
 panel: { display: "flex", flexDirection: "column", height: "100%", background: "var(--bg-primary)", color: "var(--text-primary)", fontSize: "13px" } as React.CSSProperties,
 header: { padding: "10px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)" } as React.CSSProperties,
 tabs: { display: "flex", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)" } as React.CSSProperties,
 tab: (active: boolean): React.CSSProperties => ({ padding: "6px 14px", border: "none", cursor: "pointer", fontSize: "12px", background: "none", borderBottom: active ? "2px solid var(--accent-color)" : "2px solid transparent", color: active ? "var(--text-primary)" : "var(--text-secondary)" }),
 input: { width: "100%", background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", color: "var(--text-primary)", padding: "6px 8px", borderRadius: "4px", fontSize: "12px", boxSizing: "border-box" as const } as React.CSSProperties,
 btn: { padding: "6px 12px", background: "var(--accent-color)", color: "var(--text-primary)", border: "none", borderRadius: "4px", cursor: "pointer", fontSize: "12px" } as React.CSSProperties,
 content: { flex: 1, overflow: "auto", padding: "12px" } as React.CSSProperties,
 table: { borderCollapse: "collapse" as const, width: "100%", fontSize: "12px" } as React.CSSProperties,
 th: { background: "var(--bg-secondary)", padding: "4px 8px", textAlign: "left" as const, borderBottom: "1px solid var(--border-color)" } as React.CSSProperties,
 td: { padding: "4px 8px", borderBottom: "1px solid var(--border-color)" } as React.CSSProperties,
 };

 return (
 <div style={s.panel}>
 <div style={s.header}>
 <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
 <span style={{ fontSize: "14px", fontWeight: 600 }}>Supabase</span>
 <span style={{ fontSize: "11px", padding: "2px 6px", borderRadius: "10px", background: connected ? "rgba(76,175,80,0.15)" : "color-mix(in srgb, var(--accent-rose) 15%, transparent)", color: connected ? "var(--success-color)" : "var(--error-color)" }}>
 {connected ? "Connected" : "Disconnected"}
 </span>
 </div>
 {!connected && (
 <div style={{ marginTop: "8px", display: "flex", flexDirection: "column", gap: "6px" }}>
 <input style={s.input} placeholder="Project URL (https://xxx.supabase.co)" value={config.url} onChange={e => setConfig(c => ({ ...c, url: e.target.value }))} />
 <input style={s.input} placeholder="Anon key (public)" type="password" value={config.anon_key} onChange={e => setConfig(c => ({ ...c, anon_key: e.target.value }))} />
 <button style={s.btn} onClick={saveConfig} disabled={loading}>Connect</button>
 </div>
 )}
 </div>

 {error && <div style={{ padding: "8px 12px", background: "color-mix(in srgb, var(--accent-rose) 10%, transparent)", color: "var(--error-color)", fontSize: "12px" }}>{error}</div>}

 {connected && (
 <>
 <div style={s.tabs}>
 {(["tables", "query", "ai"] as const).map(t => (
 <button key={t} style={s.tab(activeTab === t)} onClick={() => setActiveTab(t)}>
 {t === "tables" ? "Tables" : t === "query" ? "SQL" : "AI Query"}
 </button>
 ))}
 </div>

 <div style={s.content}>
 {activeTab === "tables" && (
 <div>
 <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "8px" }}>
 <span style={{ color: "var(--text-secondary)", fontSize: "11px" }}>{tables.length} table(s)</span>
 <button style={{ ...s.btn, background: "var(--bg-secondary)", color: "var(--text-secondary)" }} onClick={() => fetchTables()}>Refresh</button>
 </div>
 {tables.map(t => (
 <div key={t.name} role="button" tabIndex={0} onClick={() => selectTable(t.name)} onKeyDown={e => e.key === "Enter" && selectTable(t.name)} style={{ display: "flex", justifyContent: "space-between", padding: "8px 10px", borderRadius: "4px", cursor: "pointer", marginBottom: "2px", background: selectedTable === t.name ? "var(--bg-tertiary)" : "transparent" }}>
 <span> {t.name}</span>
 <span style={{ color: "var(--text-secondary)", fontSize: "11px" }}>{t.row_count.toLocaleString()} rows</span>
 </div>
 ))}
 {tables.length === 0 && <div style={{ color: "var(--text-secondary)", textAlign: "center", marginTop: "20px" }}>No tables found</div>}
 </div>
 )}

 {activeTab === "query" && (
 <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
 <textarea
 style={{ ...s.input, height: "120px", resize: "vertical", fontFamily: "var(--font-mono)" }}
 placeholder="SELECT * FROM users LIMIT 10"
 value={query}
 onChange={e => setQuery(e.target.value)}
 onKeyDown={e => { if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) runQuery(); }}
 />
 <button style={s.btn} onClick={runQuery} disabled={loading}>Run (Cmd+Enter)</button>
 {queryResult && !queryResult.error && (
 <div style={{ overflow: "auto" }}>
 <table style={s.table}>
 <thead><tr>{queryResult.columns.map(c => <th key={c} style={s.th}>{c}</th>)}</tr></thead>
 <tbody>{queryResult.rows.map((row, i) => <tr key={i}>{row.map((cell, j) => <td key={j} style={s.td}>{cell}</td>)}</tr>)}</tbody>
 </table>
 <div style={{ marginTop: "6px", color: "var(--text-secondary)", fontSize: "11px" }}>{queryResult.rows.length} row(s)</div>
 </div>
 )}
 </div>
 )}

 {activeTab === "ai" && (
 <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
 <p style={{ color: "var(--text-secondary)", fontSize: "12px", margin: 0 }}>Describe what you want to query in plain English:</p>
 <textarea style={{ ...s.input, height: "80px", resize: "vertical" }} placeholder="Show me all users who signed up in the last 7 days" value={naturalQuery} onChange={e => setNaturalQuery(e.target.value)} />
 <button style={s.btn} onClick={generateQuery} disabled={loading || !naturalQuery.trim()}>Generate SQL</button>
 <div style={{ marginTop: "8px" }}>
 <p style={{ color: "var(--text-secondary)", fontSize: "11px", margin: "0 0 6px" }}>Quick queries:</p>
 {[
 "Count rows in each table",
 "Show latest 20 records from selected table",
 "Find tables with most rows",
 ].map(q => (
 <div key={q} onClick={() => setNaturalQuery(q)} style={{ padding: "6px 10px", marginBottom: "4px", background: "var(--bg-secondary)", borderRadius: "4px", cursor: "pointer", fontSize: "12px" }}>{q}</div>
 ))}
 </div>
 </div>
 )}
 </div>

 <div style={{ padding: "8px 12px", borderTop: "1px solid var(--border-color)", display: "flex", gap: "8px" }}>
 <button style={{ ...s.btn, background: "var(--error-color)" }} onClick={() => { setConnected(false); setTables([]); invoke("save_supabase_config", { workspacePath, url: "", anonKey: "" }).catch(() => {}); }}>Disconnect</button>
 </div>
 </>
 )}
 </div>
 );
}
