/**
 * VibeSqlPanel — Connect to VibeSQL Server (vibesql.online).
 *
 * Supports self-hosted and cloud editions. Features:
 * - Connection manager (host, port, database, auth)
 * - Saved connections in ~/.vibeui/vibesql-connections.json
 * - Schema browser (databases → tables → columns)
 * - SQL query editor with execution
 * - Natural language → SQL via AI provider
 * - Query history
 * - Server info & status dashboard
 */
import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Key, X } from "lucide-react";

// ── Types ────────────────────────────────────────────────────────────────────

interface VibeSqlConnection {
  id: string;
  name: string;
  edition: "cloud" | "self-hosted";
  host: string;
  port: number;
  database: string;
  username: string;
  password: string;
  useSsl: boolean;
  cloudApiKey?: string;
  savedAt: string;
}

interface VibeSqlTableInfo {
  name: string;
  schema: string;
  row_count: number;
  columns: { name: string; data_type: string; nullable: boolean; primary_key: boolean }[];
}

interface VibeSqlQueryResult {
  columns: string[];
  rows: string[][];
  row_count: number;
  elapsed_ms: number;
  error?: string;
}

interface VibeSqlServerInfo {
  version: string;
  edition: string;
  uptime: string;
  databases: string[];
  connections_active: number;
  memory_used_mb: number;
}

interface QueryHistoryEntry {
  sql: string;
  timestamp: number;
  elapsed_ms: number;
  row_count: number;
  error?: string;
}

// ── Styles ───────────────────────────────────────────────────────────────────

const badge = (bg: string): React.CSSProperties => ({
  display: "inline-block", padding: "2px 8px", borderRadius: "var(--radius-md)",
  fontSize: "var(--font-size-xs)", fontWeight: 600, background: bg, color: "var(--btn-primary-fg)",
});

// ── Component ────────────────────────────────────────────────────────────────

export function VibeSqlPanel({ provider }: { workspacePath: string | null; provider: string }) {
  const [tab, setTab] = useState<"connect" | "browser" | "query" | "history" | "status">("connect");
  const [connections, setConnections] = useState<VibeSqlConnection[]>([]);
  const [activeConn, setActiveConn] = useState<VibeSqlConnection | null>(null);
  const [connected, setConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  // Connection form
  const [edition, setEdition] = useState<"cloud" | "self-hosted">("self-hosted");
  const [connName, setConnName] = useState("My VibeSQL");
  const [host, setHost] = useState("localhost");
  const [port, setPort] = useState(5432);
  const [database, setDatabase] = useState("vibesql");
  const [username, setUsername] = useState("admin");
  const [password, setPassword] = useState("");
  const [useSsl, setUseSsl] = useState(false);
  const [cloudApiKey, setCloudApiKey] = useState("");

  // Schema browser
  const [tables, setTables] = useState<VibeSqlTableInfo[]>([]);
  const [selectedTable, setSelectedTable] = useState<string | null>(null);

  // Query
  const [sql, setSql] = useState("");
  const [nlQuery, setNlQuery] = useState("");
  const [queryResult, setQueryResult] = useState<VibeSqlQueryResult | null>(null);
  const [queryLoading, setQueryLoading] = useState(false);
  const [queryHistory, setQueryHistory] = useState<QueryHistoryEntry[]>([]);

  // Server info
  const [serverInfo, setServerInfo] = useState<VibeSqlServerInfo | null>(null);

  const sqlRef = useRef<HTMLTextAreaElement>(null);

  // ── Load saved connections ──────────────────────────────────────────────────

  const loadConnections = useCallback(async () => {
    try {
      const conns = await invoke<VibeSqlConnection[]>("vibesql_list_connections");
      setConnections(conns);
    } catch { /* first run, no connections */ }
  }, []);

  useEffect(() => { loadConnections(); }, [loadConnections]);

  // ── Connection actions ──────────────────────────────────────────────────────

  const buildConnectionString = () => {
    if (edition === "cloud") {
      return `vibesql+cloud://${cloudApiKey}@cloud.vibesql.online/${database}?ssl=true`;
    }
    const ssl = useSsl ? "&sslmode=require" : "";
    return `vibesql://${username}:${password}@${host}:${port}/${database}?${ssl}`;
  };

  const handleConnect = async () => {
    setLoading(true);
    setError(null);
    try {
      const connStr = buildConnectionString();
      await invoke("vibesql_connect", { connectionString: connStr });
      const conn: VibeSqlConnection = {
        id: `vibesql-${Date.now()}`,
        name: connName,
        edition, host, port, database, username, password, useSsl, cloudApiKey,
        savedAt: new Date().toISOString(),
      };
      setActiveConn(conn);
      setConnected(true);

      // Save to persistent storage
      await invoke("vibesql_save_connection", { connection: conn }).catch(() => {});
      await loadConnections();

      // Auto-load schema
      const tbls = await invoke<VibeSqlTableInfo[]>("vibesql_list_tables", { connectionString: connStr });
      setTables(tbls);

      // Load server info
      const info = await invoke<VibeSqlServerInfo>("vibesql_server_info", { connectionString: connStr });
      setServerInfo(info);

      setTab("browser");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleDisconnect = () => {
    setConnected(false);
    setActiveConn(null);
    setTables([]);
    setQueryResult(null);
    setServerInfo(null);
    setTab("connect");
  };

  const handleQuickConnect = async (conn: VibeSqlConnection) => {
    setEdition(conn.edition);
    setConnName(conn.name);
    setHost(conn.host);
    setPort(conn.port);
    setDatabase(conn.database);
    setUsername(conn.username);
    setPassword(conn.password);
    setUseSsl(conn.useSsl);
    setCloudApiKey(conn.cloudApiKey || "");
    // Trigger connect after state updates
    setTimeout(() => handleConnect(), 100);
  };

  const handleDeleteConnection = async (id: string) => {
    try {
      await invoke("vibesql_delete_connection", { id });
      await loadConnections();
    } catch (e) { setError(String(e)); }
  };

  // ── Query execution ─────────────────────────────────────────────────────────

  const executeQuery = async () => {
    if (!sql.trim() || !activeConn) return;
    setQueryLoading(true);
    setError(null);
    try {
      const connStr = buildConnectionString();
      const result = await invoke<VibeSqlQueryResult>("vibesql_execute_query", {
        connectionString: connStr, sql: sql.trim(),
      });
      setQueryResult(result);
      setQueryHistory(prev => [{
        sql: sql.trim(), timestamp: Date.now(),
        elapsed_ms: result.elapsed_ms, row_count: result.row_count,
        error: result.error,
      }, ...prev].slice(0, 50));
    } catch (e) {
      setError(String(e));
    } finally {
      setQueryLoading(false);
    }
  };

  const generateSql = async () => {
    if (!nlQuery.trim() || !provider) return;
    setQueryLoading(true);
    try {
      const schema = tables.map(t =>
        `${t.schema}.${t.name} (${t.columns.map(c => `${c.name} ${c.data_type}${c.primary_key ? " PK" : ""}`).join(", ")})`
      ).join("\n");
      const generated = await invoke<string>("vibesql_generate_sql", {
        description: nlQuery, schema, provider,
      });
      setSql(generated);
      setNlQuery("");
      sqlRef.current?.focus();
    } catch (e) { setError(String(e)); }
    finally { setQueryLoading(false); }
  };

  // ── Tab button helper ───────────────────────────────────────────────────────

  const tabBtn = (id: string, lbl: string, disabled = false) => (
    <button key={id} disabled={disabled} onClick={() => !disabled && setTab(id as typeof tab)} style={{
      padding: "4px 12px", fontSize: "var(--font-size-sm)", fontWeight: tab === id ? 600 : 400, cursor: disabled ? "not-allowed" : "pointer",
      background: tab === id ? "var(--accent-color)" : "transparent",
      color: tab === id ? "var(--btn-primary-fg, #fff)" : disabled ? "var(--text-secondary)" : "var(--text-primary)",
      border: `1px solid ${tab === id ? "var(--accent-color)" : "var(--border-color)"}`,
      borderRadius: "var(--radius-sm)", opacity: disabled ? 0.5 : 1,
    }}>{lbl}</button>
  );

  // ── Connect tab ─────────────────────────────────────────────────────────────

  const renderConnect = () => (
    <div>
      {error && <div className="panel-error" style={{ marginBottom: 8 }}>{error}</div>}

      {/* Saved connections */}
      {connections.length > 0 && (
        <div style={{ marginBottom: 14 }}>
          <div className="panel-label" style={{ marginBottom: 8 }}>Saved Connections</div>
          {connections.map(c => (
            <div key={c.id} className="panel-card" style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
              <div style={{ flex: 1, cursor: "pointer" }} role="button" tabIndex={0}
                onClick={() => handleQuickConnect(c)} onKeyDown={e => e.key === "Enter" && handleQuickConnect(c)}>
                <div style={{ fontSize: "var(--font-size-md)", fontWeight: 600 }}>{c.name}</div>
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
                  {c.edition === "cloud" ? "cloud.vibesql.online" : `${c.host}:${c.port}`} / {c.database}
                  <span style={{ ...badge(c.edition === "cloud" ? "var(--info-color)" : "var(--success-color)"), marginLeft: 6 }}>{c.edition}</span>
                </div>
              </div>
              <button aria-label="Remove connection" onClick={() => handleDeleteConnection(c.id)}
                style={{ background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer", display: "flex", alignItems: "center" }}><X size={14} /></button>
            </div>
          ))}
        </div>
      )}

      {/* New connection form */}
      <div className="panel-label" style={{ marginBottom: 8 }}>New Connection</div>

      <div style={{ display: "flex", gap: 6, marginBottom: 12 }}>
        {(["self-hosted", "cloud"] as const).map(e => (
          <button key={e} onClick={() => setEdition(e)}
            className={`panel-btn ${e === edition ? "panel-btn-primary" : "panel-btn-secondary"}`}
            style={{ flex: 1 }}>{e === "cloud" ? "VibeSQL Cloud" : "Self-Hosted"}</button>
        ))}
      </div>

      <div style={{ marginBottom: 10 }}>
        <label className="panel-label">Connection Name</label>
        <input className="panel-input panel-input-full" value={connName} onChange={e => setConnName(e.target.value)} placeholder="My VibeSQL Server" />
      </div>

      {edition === "cloud" ? (
        <>
          <div style={{ marginBottom: 10 }}>
            <label className="panel-label">Cloud API Key</label>
            <input className="panel-input panel-input-full" style={{ fontFamily: "var(--font-mono, monospace)" }} type="password" value={cloudApiKey} onChange={e => setCloudApiKey(e.target.value)}
              placeholder="vsql_cloud_..." />
          </div>
          <div style={{ marginBottom: 10 }}>
            <label className="panel-label">Database</label>
            <input className="panel-input panel-input-full" value={database} onChange={e => setDatabase(e.target.value)} placeholder="mydb" />
          </div>
          <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 10 }}>
            Connects to <strong>cloud.vibesql.online</strong> with TLS. Get your API key at{" "}
            <span style={{ color: "var(--accent-color)" }}>vibesql.online/dashboard</span>
          </div>
        </>
      ) : (
        <>
          <div style={{ display: "flex", gap: 10, marginBottom: 10 }}>
            <div style={{ flex: 3 }}>
              <label className="panel-label">Host</label>
              <input className="panel-input panel-input-full" style={{ fontFamily: "var(--font-mono, monospace)" }} value={host} onChange={e => setHost(e.target.value)} placeholder="localhost" />
            </div>
            <div style={{ flex: 1 }}>
              <label className="panel-label">Port</label>
              <input className="panel-input panel-input-full" style={{ fontFamily: "var(--font-mono, monospace)" }} type="number" value={port} onChange={e => setPort(Number(e.target.value))} />
            </div>
          </div>
          <div style={{ marginBottom: 10 }}>
            <label className="panel-label">Database</label>
            <input className="panel-input panel-input-full" value={database} onChange={e => setDatabase(e.target.value)} placeholder="vibesql" />
          </div>
          <div style={{ display: "flex", gap: 10, marginBottom: 10 }}>
            <div style={{ flex: 1 }}>
              <label className="panel-label">Username</label>
              <input className="panel-input panel-input-full" value={username} onChange={e => setUsername(e.target.value)} placeholder="admin" />
            </div>
            <div style={{ flex: 1 }}>
              <label className="panel-label">Password</label>
              <input className="panel-input panel-input-full" style={{ fontFamily: "var(--font-mono, monospace)" }} type="password" value={password} onChange={e => setPassword(e.target.value)} />
            </div>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 12, fontSize: "var(--font-size-base)" }}>
            <input type="checkbox" id="vibesql-ssl" checked={useSsl} onChange={e => setUseSsl(e.target.checked)} />
            <label htmlFor="vibesql-ssl">Use SSL/TLS</label>
          </div>
        </>
      )}

      <button className="panel-btn panel-btn-primary panel-input-full" style={{ opacity: loading ? 0.6 : 1 }} onClick={handleConnect} disabled={loading}>
        {loading ? "Connecting..." : "Connect"}
      </button>
    </div>
  );

  // ── Schema browser tab ──────────────────────────────────────────────────────

  const selectedTableInfo = tables.find(t => t.name === selectedTable);

  const renderBrowser = () => (
    <div style={{ display: "flex", gap: 10, flex: 1, minHeight: 300 }}>
      {/* Table list */}
      <div style={{ width: 180, flexShrink: 0, borderRight: "1px solid var(--border-color)", paddingRight: 8, overflowY: "auto" }}>
        <div className="panel-label" style={{ marginBottom: 6 }}>Tables ({tables.length})</div>
        {tables.map(t => (
          <div key={t.name} role="button" tabIndex={0}
            onClick={() => setSelectedTable(t.name)} onKeyDown={e => e.key === "Enter" && setSelectedTable(t.name)}
            style={{
              padding: "8px 8px", borderRadius: "var(--radius-sm)", cursor: "pointer", marginBottom: 2, fontSize: "var(--font-size-base)",
              background: selectedTable === t.name ? "var(--accent-color)" : "transparent",
              color: selectedTable === t.name ? "var(--btn-primary-fg, #fff)" : "var(--text-primary)",
            }}>
            <div style={{ fontWeight: 500 }}>{t.name}</div>
            <div style={{ fontSize: "var(--font-size-xs)", opacity: 0.7 }}>{t.row_count.toLocaleString()} rows</div>
          </div>
        ))}
        {tables.length === 0 && <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>No tables found</div>}
      </div>

      {/* Column details */}
      <div style={{ flex: 1, overflowY: "auto" }}>
        {selectedTableInfo ? (
          <>
            <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 700, marginBottom: 8 }}>{selectedTableInfo.schema}.{selectedTableInfo.name}</div>
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 10 }}>{selectedTableInfo.row_count.toLocaleString()} rows</div>
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "var(--font-size-base)" }}>
              <thead>
                <tr style={{ borderBottom: "1px solid var(--border-color)" }}>
                  <th style={{ textAlign: "left", padding: "4px 8px", color: "var(--text-secondary)", fontWeight: 600 }}>Column</th>
                  <th style={{ textAlign: "left", padding: "4px 8px", color: "var(--text-secondary)", fontWeight: 600 }}>Type</th>
                  <th style={{ textAlign: "center", padding: "4px 8px", color: "var(--text-secondary)", fontWeight: 600 }}>PK</th>
                  <th style={{ textAlign: "center", padding: "4px 8px", color: "var(--text-secondary)", fontWeight: 600 }}>Null</th>
                </tr>
              </thead>
              <tbody>
                {selectedTableInfo.columns.map(c => (
                  <tr key={c.name} style={{ borderBottom: "1px solid var(--border-color)" }}>
                    <td style={{ padding: "4px 8px", fontWeight: c.primary_key ? 600 : 400 }}>{c.name}</td>
                    <td style={{ padding: "4px 8px", fontFamily: "var(--font-mono, monospace)", color: "var(--text-secondary)", fontSize: "var(--font-size-sm)" }}>{c.data_type}</td>
                    <td style={{ padding: "4px 8px", textAlign: "center" }}>{c.primary_key ? <Key size={12} strokeWidth={1.5} style={{ color: "var(--accent, #4a9eff)" }} /> : ""}</td>
                    <td style={{ padding: "4px 8px", textAlign: "center", color: "var(--text-secondary)" }}>{c.nullable ? "yes" : "no"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
            <div style={{ display: "flex", gap: 6, marginTop: 10 }}>
              <button className="panel-btn panel-btn-secondary" onClick={() => { setSql(`SELECT * FROM ${selectedTableInfo.schema}.${selectedTableInfo.name} LIMIT 100;`); setTab("query"); }}>
                Query Table
              </button>
              <button className="panel-btn panel-btn-secondary" onClick={() => { setSql(`SELECT COUNT(*) FROM ${selectedTableInfo.schema}.${selectedTableInfo.name};`); setTab("query"); }}>
                Count Rows
              </button>
            </div>
          </>
        ) : (
          <div style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-base)", padding: 20, textAlign: "center" }}>
            Select a table to view its schema
          </div>
        )}
      </div>
    </div>
  );

  // ── Query tab ───────────────────────────────────────────────────────────────

  const renderQuery = () => (
    <div>
      {/* Natural language input */}
      <div style={{ marginBottom: 10 }}>
        <label className="panel-label">Ask in Plain English</label>
        <div style={{ display: "flex", gap: 6 }}>
          <input className="panel-input" style={{ flex: 1 }} value={nlQuery} onChange={e => setNlQuery(e.target.value)}
            placeholder="Show top 10 customers by revenue..." onKeyDown={e => e.key === "Enter" && generateSql()} />
          <button className="panel-btn panel-btn-primary" onClick={generateSql} disabled={!nlQuery.trim() || !provider || queryLoading}>
            Generate
          </button>
        </div>
      </div>

      {/* SQL editor */}
      <div style={{ marginBottom: 10 }}>
        <label className="panel-label">SQL</label>
        <textarea ref={sqlRef} className="panel-input panel-textarea panel-input-full" style={{ fontFamily: "var(--font-mono, monospace)", minHeight: 100, resize: "vertical" }}
          value={sql} onChange={e => setSql(e.target.value)}
          placeholder="SELECT * FROM ..."
          onKeyDown={e => { if ((e.metaKey || e.ctrlKey) && e.key === "Enter") { e.preventDefault(); executeQuery(); } }} />
      </div>

      <div style={{ display: "flex", gap: 6, marginBottom: 12 }}>
        <button className="panel-btn panel-btn-primary" style={{ flex: 1 }} onClick={executeQuery} disabled={!sql.trim() || queryLoading}>
          {queryLoading ? "Running..." : "Execute (⌘+Enter)"}
        </button>
        <button className="panel-btn panel-btn-secondary" onClick={() => { setSql(""); setQueryResult(null); }}>Clear</button>
      </div>

      {error && <div className="panel-error" style={{ marginBottom: 8 }}>{error}</div>}

      {/* Results */}
      {queryResult && (
        <div>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
            <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
              {queryResult.error
                ? <span style={{ color: "var(--error-color)" }}>{queryResult.error}</span>
                : `${queryResult.row_count} row${queryResult.row_count !== 1 ? "s" : ""} in ${queryResult.elapsed_ms}ms`
              }
            </span>
          </div>
          {!queryResult.error && queryResult.columns.length > 0 && (
            <div style={{ overflowX: "auto", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)" }}>
              <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "var(--font-size-base)", fontFamily: "var(--font-mono, monospace)" }}>
                <thead>
                  <tr style={{ background: "var(--bg-tertiary)" }}>
                    {queryResult.columns.map(c => (
                      <th key={c} style={{ textAlign: "left", padding: "8px 8px", fontWeight: 600, borderBottom: "1px solid var(--border-color)", whiteSpace: "nowrap" }}>{c}</th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {queryResult.rows.map((row, i) => (
                    <tr key={i} style={{ borderBottom: "1px solid var(--border-color)" }}>
                      {row.map((cell, j) => (
                        <td key={j} style={{ padding: "4px 8px", maxWidth: 200, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}
                          title={cell}>{cell}</td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}
    </div>
  );

  // ── History tab ─────────────────────────────────────────────────────────────

  const renderHistory = () => (
    <div>
      <div className="panel-label" style={{ marginBottom: 8 }}>Query History ({queryHistory.length})</div>
      {queryHistory.length === 0 && (
        <div className="panel-empty">No queries executed yet</div>
      )}
      {queryHistory.map((h, i) => (
        <div key={i} className="panel-card" style={{ cursor: "pointer" }} role="button" tabIndex={0}
          onClick={() => { setSql(h.sql); setTab("query"); }}
          onKeyDown={e => { if (e.key === "Enter") { setSql(h.sql); setTab("query"); } }}>
          <div style={{ fontFamily: "var(--font-mono, monospace)", fontSize: "var(--font-size-sm)", marginBottom: 4, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {h.sql}
          </div>
          <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", display: "flex", gap: 8 }}>
            <span>{new Date(h.timestamp).toLocaleTimeString()}</span>
            <span>{h.row_count} rows</span>
            <span>{h.elapsed_ms}ms</span>
            {h.error && <span style={{ color: "var(--error-color)" }}>Error</span>}
          </div>
        </div>
      ))}
    </div>
  );

  // ── Status tab ──────────────────────────────────────────────────────────────

  const renderStatus = () => (
    <div>
      {serverInfo ? (
        <>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, marginBottom: 12 }}>
            <div className="panel-card">
              <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", textTransform: "uppercase", marginBottom: 2 }}>Version</div>
              <div style={{ fontSize: 16, fontWeight: 700, fontFamily: "var(--font-mono, monospace)" }}>{serverInfo.version}</div>
            </div>
            <div className="panel-card">
              <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", textTransform: "uppercase", marginBottom: 2 }}>Edition</div>
              <div style={{ fontSize: 16, fontWeight: 700 }}>
                {serverInfo.edition}
                <span style={{ ...badge(serverInfo.edition.toLowerCase().includes("cloud") ? "var(--info-color)" : "var(--success-color)"), marginLeft: 6, fontSize: 9 }}>
                  {serverInfo.edition.toLowerCase().includes("cloud") ? "CLOUD" : "SELF-HOSTED"}
                </span>
              </div>
            </div>
            <div className="panel-card">
              <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", textTransform: "uppercase", marginBottom: 2 }}>Uptime</div>
              <div style={{ fontSize: 16, fontWeight: 700, fontFamily: "var(--font-mono, monospace)" }}>{serverInfo.uptime}</div>
            </div>
            <div className="panel-card">
              <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", textTransform: "uppercase", marginBottom: 2 }}>Connections</div>
              <div style={{ fontSize: 16, fontWeight: 700, fontFamily: "var(--font-mono, monospace)" }}>{serverInfo.connections_active}</div>
            </div>
          </div>
          <div className="panel-card">
            <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", textTransform: "uppercase", marginBottom: 4 }}>Memory</div>
            <div style={{ fontSize: "var(--font-size-md)", fontFamily: "var(--font-mono, monospace)" }}>{serverInfo.memory_used_mb} MB</div>
          </div>
          {serverInfo.databases.length > 0 && (
            <div className="panel-card" style={{ marginTop: 8 }}>
              <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", textTransform: "uppercase", marginBottom: 6 }}>Databases</div>
              <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
                {serverInfo.databases.map(db => (
                  <span key={db} style={{ ...badge("var(--accent-color)"), fontSize: "var(--font-size-sm)" }}>{db}</span>
                ))}
              </div>
            </div>
          )}
        </>
      ) : (
        <div className="panel-empty">Connect to a server to view status</div>
      )}
    </div>
  );

  // ── Layout ──────────────────────────────────────────────────────────────────

  return (
    <div className="panel-container">
      <div className="panel-header">
        <h3>VibeSQL</h3>
        {connected && activeConn && (
          <span style={{ ...badge("var(--success-color)"), fontSize: 9 }}>
            {activeConn.name}
          </span>
        )}
        <div style={{ display: "flex", gap: 4, alignItems: "center", marginLeft: "auto" }}>
          {connected && (
            <button className="panel-btn panel-btn-secondary panel-btn-xs" onClick={handleDisconnect}>Disconnect</button>
          )}
          {tabBtn("connect", "Connect")}
          {tabBtn("browser", "Schema", !connected)}
          {tabBtn("query", "Query", !connected)}
          {tabBtn("history", "History")}
          {tabBtn("status", "Status", !connected)}
        </div>
      </div>
      <div className="panel-body">
        {tab === "connect" && renderConnect()}
        {tab === "browser" && renderBrowser()}
        {tab === "query" && renderQuery()}
        {tab === "history" && renderHistory()}
        {tab === "status" && renderStatus()}
      </div>
    </div>
  );
}

export default VibeSqlPanel;
