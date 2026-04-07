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

 return (
 <div className="panel-container">
 <div className="panel-header">
 <h3>Supabase</h3>
 <span style={{ fontSize: "11px", padding: "2px 6px", borderRadius: "10px", background: connected ? "rgba(76,175,80,0.15)" : "color-mix(in srgb, var(--accent-rose) 15%, transparent)", color: connected ? "var(--success-color)" : "var(--error-color)" }}>
 {connected ? "Connected" : "Disconnected"}
 </span>
 </div>
 {!connected && (
 <div style={{ marginTop: "8px", display: "flex", flexDirection: "column", gap: "6px" }}>
 <input className="panel-input panel-input-full" placeholder="Project URL (https://xxx.supabase.co)" value={config.url} onChange={e => setConfig(c => ({ ...c, url: e.target.value }))} />
 <input className="panel-input panel-input-full" placeholder="Anon key (public)" type="password" value={config.anon_key} onChange={e => setConfig(c => ({ ...c, anon_key: e.target.value }))} />
 <button className="panel-btn panel-btn-primary" onClick={saveConfig} disabled={loading}>Connect</button>
 </div>
 )}

 {error && <div className="panel-error">{error}</div>}

 {connected && (
 <>
 <div className="panel-tab-bar">
 {(["tables", "query", "ai"] as const).map(t => (
 <button key={t} className={`panel-tab ${activeTab === t ? "active" : ""}`} onClick={() => setActiveTab(t)}>
 {t === "tables" ? "Tables" : t === "query" ? "SQL" : "AI Query"}
 </button>
 ))}
 </div>

 <div className="panel-body">
 {activeTab === "tables" && (
 <div>
 <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "8px" }}>
 <span style={{ color: "var(--text-secondary)", fontSize: "11px" }}>{tables.length} table(s)</span>
 <button className="panel-btn panel-btn-secondary" onClick={() => fetchTables()}>Refresh</button>
 </div>
 {tables.map(t => (
 <div key={t.name} role="button" tabIndex={0} onClick={() => selectTable(t.name)} onKeyDown={e => e.key === "Enter" && selectTable(t.name)} className="panel-card" style={{ display: "flex", justifyContent: "space-between", cursor: "pointer", background: selectedTable === t.name ? "var(--bg-tertiary)" : undefined }}>
 <span> {t.name}</span>
 <span style={{ color: "var(--text-secondary)", fontSize: "11px" }}>{t.row_count.toLocaleString()} rows</span>
 </div>
 ))}
 {tables.length === 0 && <div className="panel-empty">No tables found</div>}
 </div>
 )}

 {activeTab === "query" && (
 <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
 <textarea
 className="panel-input panel-textarea panel-input-full"
 style={{ height: "120px", resize: "vertical", fontFamily: "var(--font-mono)" }}
 placeholder="SELECT * FROM users LIMIT 10"
 value={query}
 onChange={e => setQuery(e.target.value)}
 onKeyDown={e => { if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) runQuery(); }}
 />
 <button className="panel-btn panel-btn-primary" onClick={runQuery} disabled={loading}>Run (Cmd+Enter)</button>
 {queryResult && !queryResult.error && (
 <div style={{ overflow: "auto" }}>
 <table className="panel-table">
 <thead><tr>{queryResult.columns.map(c => <th key={c}>{c}</th>)}</tr></thead>
 <tbody>{queryResult.rows.map((row, i) => <tr key={i}>{row.map((cell, j) => <td key={j}>{cell}</td>)}</tr>)}</tbody>
 </table>
 <div style={{ marginTop: "6px", color: "var(--text-secondary)", fontSize: "11px" }}>{queryResult.rows.length} row(s)</div>
 </div>
 )}
 </div>
 )}

 {activeTab === "ai" && (
 <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
 <p style={{ color: "var(--text-secondary)", fontSize: "12px", margin: 0 }}>Describe what you want to query in plain English:</p>
 <textarea className="panel-input panel-textarea panel-input-full" style={{ height: "80px", resize: "vertical" }} placeholder="Show me all users who signed up in the last 7 days" value={naturalQuery} onChange={e => setNaturalQuery(e.target.value)} />
 <button className="panel-btn panel-btn-primary" onClick={generateQuery} disabled={loading || !naturalQuery.trim()}>Generate SQL</button>
 <div style={{ marginTop: "8px" }}>
 <p style={{ color: "var(--text-secondary)", fontSize: "11px", margin: "0 0 6px" }}>Quick queries:</p>
 {[
 "Count rows in each table",
 "Show latest 20 records from selected table",
 "Find tables with most rows",
 ].map(q => (
 <div key={q} onClick={() => setNaturalQuery(q)} className="panel-card" style={{ cursor: "pointer" }}>{q}</div>
 ))}
 </div>
 </div>
 )}
 </div>

 <div className="panel-footer">
 <button className="panel-btn panel-btn-danger" onClick={() => { setConnected(false); setTables([]); invoke("save_supabase_config", { workspacePath, url: "", anonKey: "" }).catch(() => {}); }}>Disconnect</button>
 </div>
 </>
 )}
 </div>
 );
}
