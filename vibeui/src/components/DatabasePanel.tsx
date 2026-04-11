/**
 * DatabasePanel — built-in database browser with AI-assisted queries.
 *
 * Supports: SQLite (auto-detect), PostgreSQL (connection string), Supabase (URL + key)
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useToast } from "../hooks/useToast";
import { Toaster } from "./Toaster";

interface TableInfo {
 name: string;
 row_count: number;
 columns: ColumnInfo[];
}

interface ColumnInfo {
 name: string;
 data_type: string;
 nullable: boolean;
 primary_key: boolean;
}

interface QueryResult {
 columns: string[];
 rows: Record<string, unknown>[];
 row_count: number;
 error: string | null;
}

type DbType = "sqlite" | "postgres" | "supabase";

interface DatabasePanelProps {
 workspacePath: string | null;
 provider: string;
}

export function DatabasePanel({ workspacePath, provider }: DatabasePanelProps) {
 const { toasts, toast, dismiss } = useToast();
 const [dbType, setDbType] = useState<DbType>("sqlite");
 const [connectionString, setConnectionString] = useState("");
 const [tables, setTables] = useState<TableInfo[]>([]);
 const [selectedTable, setSelectedTable] = useState<string | null>(null);
 const [queryResult, setQueryResult] = useState<QueryResult | null>(null);
 const [sqlQuery, setSqlQuery] = useState("");
 const [nlQuery, setNlQuery] = useState("");
 const [isConnected, setIsConnected] = useState(false);
 const [isLoading, setIsLoading] = useState(false);
 const PAGE_SIZE = 50;

 useEffect(() => {
 // Auto-detect SQLite files in workspace
 if (dbType === "sqlite" && workspacePath) {
 invoke<string[]>("find_sqlite_files", { workspacePath })
 .then((files) => {
 if (files.length > 0) setConnectionString(files[0]);
 })
 .catch(() => null);
 }
 }, [dbType, workspacePath]);

 if (!workspacePath) {
 return <div className="empty-state"><p>Open a workspace folder to use the database browser.</p></div>;
 }

 const handleConnect = async () => {
 setIsLoading(true);
 try {
 const result = await invoke<TableInfo[]>("list_db_tables", { connectionString, dbType });
 setTables(result);
 setIsConnected(true);
 } catch (e) {
 toast.error(`Connection failed: ${e}`);
 } finally {
 setIsLoading(false);
 }
 };

 const handleTableClick = async (tableName: string) => {
 setSelectedTable(tableName);
 const sql = `SELECT * FROM "${tableName}"LIMIT ${PAGE_SIZE} OFFSET 0`;
 setSqlQuery(sql);
 await runQuery(sql);
 };

 const runQuery = async (sql: string) => {
 setIsLoading(true);
 try {
 const result = await invoke<QueryResult>("query_db", { connectionString, dbType, sql });
 setQueryResult(result);
 } catch (e) {
 setQueryResult({ columns: [], rows: [], row_count: 0, error: String(e) });
 } finally {
 setIsLoading(false);
 }
 };

 const handleNlQuery = async () => {
 if (!nlQuery.trim()) return;
 setIsLoading(true);
 try {
 // Use LLM to generate SQL from NL description
 const schema = tables.map(t =>
 `${t.name}(${t.columns.map(c => `${c.name} ${c.data_type}${c.primary_key ? "PK" : ""}`).join(", ")})`
 ).join("\n");

 const sql = await invoke<string>("generate_sql_query", {
 description: nlQuery,
 schema,
 provider,
 });
 setSqlQuery(sql);
 await runQuery(sql);
 } finally {
 setIsLoading(false);
 }
 };

 const handleGenerateMigration = async () => {
 const desc = prompt("Describe the migration (e.g., 'Add email column to users table'):");
 if (!desc) return;
 setIsLoading(true);
 try {
 const migration = await invoke<string>("generate_migration", {
 connectionString,
 dbType,
 description: desc,
 provider,
 });
 setSqlQuery(migration);
 } finally {
 setIsLoading(false);
 }
 };

 return (
 <div className="panel-container" style={{ flexDirection: "row" }}>
 {/* Left: Tables list */}
 <div style={{ width: 200, flexShrink: 0, borderRight: "1px solid var(--border-color)", display: "flex", flexDirection: "column" }}>
 {/* Connection area */}
 <div style={{ padding: 12, borderBottom: "1px solid var(--border-color)" }}>
 <div role="tablist" style={{ display: "flex", gap: 4, marginBottom: 8 }}>
 {(["sqlite", "postgres", "supabase"] as DbType[]).map((t) => (
 <button
 key={t}
 role="tab"
 aria-selected={dbType === t}
 onClick={() => { setDbType(t); setIsConnected(false); setTables([]); }}
 style={{
 flex: 1,
 background: dbType === t ? "var(--accent-color)" : "var(--bg-secondary)",
 border: "none",
 borderRadius: 4,
 padding: "3px 0",
 fontSize: 10,
 cursor: "pointer",
 color: "var(--text-primary)",
 fontWeight: dbType === t ? 600 : 400,
 }}
 >
 {t === "sqlite" ? "SQLite" : t === "postgres" ? "PG" : "SB"}
 </button>
 ))}
 </div>
 <input
 value={connectionString}
 onChange={(e) => setConnectionString(e.target.value)}
 placeholder={dbType === "sqlite" ? "path/to/db.sqlite" : dbType === "postgres" ? "postgresql://..." : "https://xxx.supabase.co"}
 className="panel-input panel-input-full"
 style={{ marginBottom: 6 }}
 />
 <button
 onClick={handleConnect}
 disabled={isLoading}
 className="panel-btn panel-btn-primary"
 style={{ width: "100%" }}
 >
 {isConnected ? "Reconnect" : "Connect"}
 </button>
 </div>

 {/* Tables */}
 <div style={{ flex: 1, overflowY: "auto", padding: 8 }}>
 {tables.length === 0 && isConnected && (
 <div style={{ fontSize: 11, opacity: 0.5 }}>No tables found</div>
 )}
 {tables.map((t) => (
 <button
 key={t.name}
 onClick={() => handleTableClick(t.name)}
 style={{
 display: "block",
 width: "100%",
 textAlign: "left",
 background: selectedTable === t.name ? "var(--bg-tertiary)" : "none",
 border: "none",
 borderRadius: 4,
 padding: "5px 8px",
 cursor: "pointer",
 color: "var(--text-primary)",
 fontSize: 12,
 marginBottom: 2,
 }}
 >
 <div> {t.name}</div>
 <div style={{ fontSize: 10, opacity: 0.5 }}>{t.row_count.toLocaleString()} rows</div>
 </button>
 ))}
 </div>
 </div>

 {/* Right: Query + Results */}
 <div role="tabpanel" style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column" }}>
 {/* Query toolbar */}
 <div style={{ padding: 12, borderBottom: "1px solid var(--border-color)", display: "flex", flexDirection: "column", gap: 8 }}>
 {/* NL query */}
 <div style={{ display: "flex", gap: 8 }}>
 <input
 value={nlQuery}
 onChange={(e) => setNlQuery(e.target.value)}
 onKeyDown={(e) => e.key === "Enter" && handleNlQuery()}
 placeholder="Ask in plain English (e.g., 'Show users signed up this week')"
 className="panel-input panel-input-full"
 style={{ flex: 1 }}
 />
 <button onClick={handleNlQuery} disabled={isLoading} className="panel-btn panel-btn-primary panel-btn-sm">Ask AI</button>
 </div>
 {/* SQL editor */}
 <div style={{ display: "flex", gap: 8 }}>
 <textarea
 value={sqlQuery}
 onChange={(e) => setSqlQuery(e.target.value)}
 rows={2}
 placeholder="SELECT * FROM users LIMIT 50"
 className="panel-input panel-textarea panel-input-full"
 style={{ flex: 1, fontFamily: "var(--font-mono)", resize: "none" }}
 />
 <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
 <button onClick={() => runQuery(sqlQuery)} disabled={isLoading || !sqlQuery.trim()} className="panel-btn panel-btn-primary panel-btn-xs">Run</button>
 <button onClick={handleGenerateMigration} disabled={isLoading} className="panel-btn panel-btn-secondary panel-btn-xs">+ Migration</button>
 </div>
 </div>
 </div>

 {/* Results table */}
 <div style={{ flex: 1, overflow: "auto", padding: 12 }}>
 {isLoading && <div className="panel-loading">Loading…</div>}
 {queryResult?.error && (
 <div className="panel-error" role="alert" style={{ fontFamily: "var(--font-mono)", fontSize: 12, padding: 8 }}>
 {queryResult.error}
 </div>
 )}
 {queryResult && !queryResult.error && queryResult.rows.length > 0 && (
 <>
 <div style={{ fontSize: 11, opacity: 0.5, marginBottom: 8 }}>{queryResult.row_count} rows</div>
 <div style={{ overflowX: "auto" }}>
 <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12, fontFamily: "var(--font-mono)" }}>
 <thead>
 <tr style={{ background: "var(--bg-secondary)" }}>
 {queryResult.columns.map((col) => (
 <th key={col} scope="col" style={{ padding: "4px 8px", textAlign: "left", borderBottom: "1px solid var(--border-color)", fontWeight: 600, whiteSpace: "nowrap" }}>{col}</th>
 ))}
 </tr>
 </thead>
 <tbody>
 {queryResult.rows.slice(0, PAGE_SIZE).map((row, i) => (
 <tr key={i} style={{ background: i % 2 === 0 ? "transparent" : "var(--bg-secondary)" }}>
 {queryResult.columns.map((col) => (
 <td key={col} style={{ padding: "3px 8px", borderBottom: "1px solid var(--border-color)", opacity: row[col] === null ? 0.3 : 1, maxWidth: 200, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
 {row[col] === null ? "NULL" : String(row[col])}
 </td>
 ))}
 </tr>
 ))}
 </tbody>
 </table>
 </div>
 </>
 )}
 {queryResult && !queryResult.error && queryResult.rows.length === 0 && (
 <div style={{ opacity: 0.5, fontSize: 12 }}>No rows returned</div>
 )}
 </div>
 </div>
 <Toaster toasts={toasts} onDismiss={dismiss} />
 </div>
 );
}
