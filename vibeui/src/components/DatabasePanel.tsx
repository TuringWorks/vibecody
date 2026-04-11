/**
 * DatabasePanel — DataGrip-class database browser with AI-assisted queries.
 *
 * Supports 35+ databases: SQLite, DuckDB, PostgreSQL family, MySQL family,
 * MSSQL, MongoDB, Redis, Cassandra, Elasticsearch, Snowflake, BigQuery, and more.
 * Features: saved profiles, schema tree, NL→SQL, export CSV/JSON, query history.
 */
import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2 } from "lucide-react";
import { useToast } from "../hooks/useToast";
import { Toaster } from "./Toaster";

// ─── Types ────────────────────────────────────────────────────────────────────

type DbDriver =
  | "sqlite" | "duckdb"
  | "postgres" | "cockroachdb" | "neon" | "supabase" | "redshift" | "alloydb"
  | "aurora-pg" | "timescaledb" | "yugabytedb"
  | "mysql" | "mariadb" | "planetscale" | "tidb" | "singlestore" | "aurora-mysql"
  | "mssql" | "azure-sql"
  | "mongodb" | "mongodb-atlas"
  | "redis" | "valkey" | "upstash"
  | "cassandra" | "scylladb"
  | "elasticsearch" | "opensearch"
  | "clickhouse" | "clickhouse-cloud" | "snowflake" | "bigquery";

interface DbConnectionParams {
  driver: DbDriver;
  filepath?: string;
  host?: string;
  port?: number;
  database?: string;
  username?: string;
  password?: string;
  ssl?: boolean;
  connection_string?: string;
  account?: string;
  warehouse?: string;
  project?: string;
  dataset?: string;
  token?: string;
  url?: string;
  region?: string;
  keyspace?: string;
  index?: string;
}

interface DbSavedProfile {
  id: string;
  name: string;
  driver: DbDriver;
  params: DbConnectionParams;
  created_at: number;
  last_used?: number;
}

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

interface LocalDbFile {
  path: string;
  driver: "sqlite" | "duckdb";
  name: string;
}

interface DatabasePanelProps {
  workspacePath: string | null;
  provider: string;
}

// ─── DB Catalog ───────────────────────────────────────────────────────────────

interface CatalogEntry {
  driver: DbDriver;
  label: string;
  category: string;
  requiresCli?: string;
  defaultPort?: number;
  icon: string;
}

const DB_CATALOG: CatalogEntry[] = [
  { driver: "sqlite",           label: "SQLite",           category: "File-based",  icon: "🗄️" },
  { driver: "duckdb",           label: "DuckDB",           category: "File-based",  icon: "🦆" },
  { driver: "postgres",         label: "PostgreSQL",       category: "Relational",  icon: "🐘", defaultPort: 5432 },
  { driver: "cockroachdb",      label: "CockroachDB",      category: "Relational",  icon: "🪳", defaultPort: 26257 },
  { driver: "timescaledb",      label: "TimescaleDB",      category: "Relational",  icon: "⏱️", defaultPort: 5432 },
  { driver: "yugabytedb",       label: "YugabyteDB",       category: "Relational",  icon: "🌍", defaultPort: 5433 },
  { driver: "mysql",            label: "MySQL",            category: "Relational",  icon: "🐬", defaultPort: 3306 },
  { driver: "mariadb",          label: "MariaDB",          category: "Relational",  icon: "🦭", defaultPort: 3306 },
  { driver: "tidb",             label: "TiDB",             category: "Relational",  icon: "🔶", defaultPort: 4000 },
  { driver: "singlestore",      label: "SingleStore",      category: "Relational",  icon: "💎", defaultPort: 3306 },
  { driver: "mssql",            label: "SQL Server",       category: "Relational",  icon: "🪟", defaultPort: 1433 },
  { driver: "neon",             label: "Neon",             category: "Cloud SQL",   icon: "✨", defaultPort: 5432 },
  { driver: "supabase",         label: "Supabase",         category: "Cloud SQL",   icon: "⚡", defaultPort: 5432 },
  { driver: "alloydb",          label: "AlloyDB",          category: "Cloud SQL",   icon: "🔷", defaultPort: 5432 },
  { driver: "aurora-pg",        label: "Aurora (PG)",      category: "Cloud SQL",   icon: "🔷", defaultPort: 5432 },
  { driver: "aurora-mysql",     label: "Aurora (MySQL)",   category: "Cloud SQL",   icon: "🟠", defaultPort: 3306 },
  { driver: "planetscale",      label: "PlanetScale",      category: "Cloud SQL",   icon: "🪐", defaultPort: 3306 },
  { driver: "azure-sql",        label: "Azure SQL",        category: "Cloud SQL",   icon: "🔵", defaultPort: 1433 },
  { driver: "redshift",         label: "Amazon Redshift",  category: "Analytics",   icon: "🔴", defaultPort: 5439 },
  { driver: "clickhouse",       label: "ClickHouse",       category: "Analytics",   icon: "🖱️", defaultPort: 9000 },
  { driver: "clickhouse-cloud", label: "ClickHouse Cloud", category: "Analytics",   icon: "☁️", defaultPort: 9440 },
  { driver: "snowflake",        label: "Snowflake",        category: "Analytics",   icon: "❄️" },
  { driver: "bigquery",         label: "BigQuery",         category: "Analytics",   icon: "📊" },
  { driver: "mongodb",          label: "MongoDB",          category: "NoSQL",       icon: "🍃", defaultPort: 27017 },
  { driver: "mongodb-atlas",    label: "MongoDB Atlas",    category: "NoSQL",       icon: "🌿" },
  { driver: "redis",            label: "Redis",            category: "NoSQL",       icon: "🔴", defaultPort: 6379 },
  { driver: "valkey",           label: "Valkey",           category: "NoSQL",       icon: "🔑", defaultPort: 6379 },
  { driver: "upstash",          label: "Upstash Redis",    category: "Cloud NoSQL", icon: "⬆️" },
  { driver: "cassandra",        label: "Cassandra",        category: "NoSQL",       icon: "💠", defaultPort: 9042 },
  { driver: "scylladb",         label: "ScyllaDB",         category: "NoSQL",       icon: "🪸", defaultPort: 9042 },
  { driver: "elasticsearch",    label: "Elasticsearch",    category: "Search",      icon: "🔍", defaultPort: 9200 },
  { driver: "opensearch",       label: "OpenSearch",       category: "Search",      icon: "🔎", defaultPort: 9200 },
];

const CATEGORIES = Array.from(new Set(DB_CATALOG.map((d) => d.category)));

function catalogEntry(driver: DbDriver): CatalogEntry {
  return DB_CATALOG.find((e) => e.driver === driver) ?? {
    driver, label: driver, category: "Other", icon: "🗃️",
  };
}

// ─── Connection status ────────────────────────────────────────────────────────

type ConnectionStatus = "disconnected" | "connecting" | "connected" | "error";

interface ActiveConnection {
  profile: DbSavedProfile;
  status: ConnectionStatus;
  tables: TableInfo[];
}

// ─── Utility ──────────────────────────────────────────────────────────────────

function generateProfileName(driver: DbDriver, params: DbConnectionParams): string {
  const entry = catalogEntry(driver);
  if (driver === "sqlite" || driver === "duckdb") {
    const parts = (params.filepath ?? "").split(/[/\\]/);
    return `${entry.label} · ${parts[parts.length - 1] || "local"}`;
  }
  if (params.host) return `${entry.label} · ${params.host}`;
  if (params.url) {
    try { return `${entry.label} · ${new URL(params.url).hostname}`; } catch { /* ok */ }
  }
  if (params.account) return `${entry.label} · ${params.account}`;
  if (params.project) return `${entry.label} · ${params.project}`;
  return `${entry.label} · new`;
}

function exportBlob(data: string, filename: string, type: string) {
  const blob = new Blob([data], { type });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

function toCSV(columns: string[], rows: Record<string, unknown>[]): string {
  const header = columns.map((c) => JSON.stringify(c)).join(",");
  const body = rows
    .map((row) => columns.map((c) => JSON.stringify(row[c] ?? "")).join(","))
    .join("\n");
  return `${header}\n${body}`;
}

// ─── Sub-components ───────────────────────────────────────────────────────────

function StatusDot({ status }: { status: ConnectionStatus }) {
  const color =
    status === "connected" ? "var(--success-color, #22c55e)" :
    status === "connecting" ? "#f59e0b" :
    status === "error" ? "var(--error-color, #ef4444)" :
    "var(--text-muted, #6b7280)";
  return (
    <span
      aria-label={status}
      style={{
        display: "inline-block",
        width: 8,
        height: 8,
        borderRadius: "50%",
        background: color,
        flexShrink: 0,
        boxShadow: status === "connected" ? `0 0 4px ${color}` : "none",
      }}
    />
  );
}

// ─── Connection Wizard ────────────────────────────────────────────────────────

interface WizardProps {
  workspacePath: string | null;
  onClose: () => void;
  onSaved: (profile: DbSavedProfile) => void;
  toast: { success: (m: string) => void; error: (m: string) => void; info: (m: string) => void; warn: (m: string) => void };
}

function ConnectionWizard({ workspacePath, onClose, onSaved, toast }: WizardProps) {
  const [step, setStep] = useState<1 | 2>(1);
  const [selectedDriver, setSelectedDriver] = useState<DbDriver | null>(null);
  const [params, setParams] = useState<DbConnectionParams>({ driver: "sqlite" });
  const [profileName, setProfileName] = useState("");
  const [testStatus, setTestStatus] = useState<"idle" | "testing" | "ok" | "fail">("idle");
  const [testMessage, setTestMessage] = useState("");
  const [localFiles, setLocalFiles] = useState<LocalDbFile[]>([]);
  const [isSaving, setIsSaving] = useState(false);

  // Load local DB files when file-based driver is selected
  useEffect(() => {
    if (!workspacePath) return;
    if (selectedDriver === "sqlite" || selectedDriver === "duckdb") {
      invoke<LocalDbFile[]>("db_find_local_files", { workspacePath })
        .then(setLocalFiles)
        .catch(() => setLocalFiles([]));
    }
  }, [selectedDriver, workspacePath]);

  const selectDriver = (driver: DbDriver) => {
    const entry = catalogEntry(driver);
    setSelectedDriver(driver);
    setParams({
      driver,
      port: entry.defaultPort,
      ssl: driver !== "sqlite" && driver !== "duckdb" && driver !== "redis" && driver !== "valkey",
    });
    setProfileName(generateProfileName(driver, { driver }));
    setStep(2);
    setTestStatus("idle");
  };

  const updateParam = <K extends keyof DbConnectionParams>(key: K, value: DbConnectionParams[K]) => {
    setParams((prev) => {
      const next = { ...prev, [key]: value };
      if (key === "host" || key === "filepath" || key === "url" || key === "account" || key === "project") {
        setProfileName(generateProfileName(next.driver, next));
      }
      return next;
    });
  };

  const handleTestConnection = async () => {
    setTestStatus("testing");
    setTestMessage("");
    try {
      const msg = await invoke<string>("db_test_connection", { params });
      setTestStatus("ok");
      setTestMessage(msg);
    } catch (e) {
      setTestStatus("fail");
      setTestMessage(String(e));
    }
  };

  const handleSaveAndConnect = async () => {
    if (!workspacePath) { toast.warn("No workspace open."); return; }
    if (!profileName.trim()) { toast.warn("Enter a connection name."); return; }
    setIsSaving(true);
    try {
      const profile: DbSavedProfile = {
        id: `db-${Date.now()}`,
        name: profileName.trim(),
        driver: params.driver,
        params,
        created_at: Date.now(),
      };
      await invoke<string>("db_save_profile", { workspacePath, profile });
      toast.success(`Saved: ${profile.name}`);
      onSaved(profile);
      onClose();
    } catch (e) {
      toast.error(`Save failed: ${e}`);
    } finally {
      setIsSaving(false);
    }
  };

  const isFileBased = selectedDriver === "sqlite" || selectedDriver === "duckdb";
  const isKV = selectedDriver === "redis" || selectedDriver === "valkey";
  const isElastic = selectedDriver === "elasticsearch" || selectedDriver === "opensearch";
  const isSnowflake = selectedDriver === "snowflake";
  const isBigQuery = selectedDriver === "bigquery";
  const isMongoDB = selectedDriver === "mongodb-atlas";
  const isTurso = selectedDriver === "upstash";
  const isCloudString = selectedDriver === "neon" || selectedDriver === "supabase" ||
    selectedDriver === "planetscale" || selectedDriver === "mongodb-atlas" || selectedDriver === "upstash";

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="New Database Connection"
      style={{
        position: "absolute",
        inset: 0,
        background: "rgba(0,0,0,0.6)",
        zIndex: 50,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
      }}
    >
      <div
        style={{
          background: "var(--bg-primary, #1e1e2e)",
          border: "1px solid var(--border-color, #3a3a5c)",
          borderRadius: 10,
          width: "min(820px, 96vw)",
          maxHeight: "90vh",
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
        }}
      >
        {/* Header */}
        <div
          style={{
            padding: "14px 20px",
            borderBottom: "1px solid var(--border-color, #3a3a5c)",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
          }}
        >
          <h2 style={{ margin: 0, fontSize: 15, fontWeight: 600, color: "var(--text-primary)" }}>
            {step === 1 ? "Select Database Type" : `Configure ${selectedDriver ? catalogEntry(selectedDriver).label : ""}`}
          </h2>
          <button
            onClick={onClose}
            aria-label="Close wizard"
            style={{
              background: "none",
              border: "none",
              color: "var(--text-muted)",
              cursor: "pointer",
              fontSize: 18,
              lineHeight: 1,
            }}
          >
            ×
          </button>
        </div>

        {/* Step 1: Driver grid */}
        {step === 1 && (
          <div style={{ flex: 1, overflowY: "auto", padding: 20 }}>
            {CATEGORIES.map((cat) => {
              const entries = DB_CATALOG.filter((e) => e.category === cat);
              return (
                <div key={cat} style={{ marginBottom: 20 }}>
                  <div
                    style={{
                      fontSize: 10,
                      fontWeight: 700,
                      textTransform: "uppercase",
                      letterSpacing: "0.08em",
                      color: "var(--text-muted)",
                      marginBottom: 8,
                    }}
                  >
                    {cat}
                  </div>
                  <div
                    style={{
                      display: "grid",
                      gridTemplateColumns: "repeat(4, 1fr)",
                      gap: 8,
                    }}
                  >
                    {entries.map((e) => (
                      <button
                        key={e.driver}
                        onClick={() => selectDriver(e.driver)}
                        style={{
                          display: "flex",
                          alignItems: "center",
                          gap: 8,
                          padding: "9px 12px",
                          background: "var(--bg-secondary, #2a2a3e)",
                          border: "1px solid var(--border-color, #3a3a5c)",
                          borderRadius: 6,
                          cursor: "pointer",
                          color: "var(--text-primary)",
                          fontSize: 12,
                          fontWeight: 500,
                          textAlign: "left",
                          transition: "border-color 0.15s",
                        }}
                        onMouseEnter={(ev) => { (ev.currentTarget as HTMLButtonElement).style.borderColor = "var(--accent-color, #7c3aed)"; }}
                        onMouseLeave={(ev) => { (ev.currentTarget as HTMLButtonElement).style.borderColor = "var(--border-color, #3a3a5c)"; }}
                      >
                        <span style={{ fontSize: 16 }}>{e.icon}</span>
                        <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{e.label}</span>
                      </button>
                    ))}
                  </div>
                </div>
              );
            })}
          </div>
        )}

        {/* Step 2: Form */}
        {step === 2 && selectedDriver && (
          <div style={{ flex: 1, overflowY: "auto", padding: 20, display: "flex", flexDirection: "column", gap: 14 }}>
            {/* Connection name */}
            <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
              <span style={{ fontSize: 11, color: "var(--text-muted)", fontWeight: 600 }}>Connection name</span>
              <input
                className="panel-input panel-input-full"
                value={profileName}
                onChange={(e) => setProfileName(e.target.value)}
                placeholder="My PostgreSQL"
              />
            </label>

            {/* File-based */}
            {isFileBased && (
              <>
                <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                  <span style={{ fontSize: 11, color: "var(--text-muted)", fontWeight: 600 }}>File path</span>
                  <input
                    className="panel-input panel-input-full"
                    value={params.filepath ?? ""}
                    onChange={(e) => updateParam("filepath", e.target.value)}
                    placeholder={selectedDriver === "sqlite" ? "/path/to/database.db" : "/path/to/database.duckdb"}
                  />
                </label>
                {localFiles.length > 0 && (
                  <div>
                    <div style={{ fontSize: 11, color: "var(--text-muted)", marginBottom: 6, fontWeight: 600 }}>
                      Found in workspace:
                    </div>
                    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                      {localFiles
                        .filter((f) => f.driver === selectedDriver)
                        .map((f) => (
                          <button
                            key={f.path}
                            onClick={() => updateParam("filepath", f.path)}
                            style={{
                              textAlign: "left",
                              background: params.filepath === f.path ? "var(--bg-tertiary)" : "var(--bg-secondary)",
                              border: "1px solid var(--border-color)",
                              borderRadius: 4,
                              padding: "5px 10px",
                              fontSize: 11,
                              color: "var(--text-primary)",
                              cursor: "pointer",
                              fontFamily: "var(--font-mono)",
                            }}
                          >
                            {f.name} <span style={{ opacity: 0.5 }}>{f.path}</span>
                          </button>
                        ))}
                    </div>
                  </div>
                )}
              </>
            )}

            {/* Cloud connection string (primary) */}
            {isCloudString && !isTurso && (
              <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                <span style={{ fontSize: 11, color: "var(--text-muted)", fontWeight: 600 }}>Connection string</span>
                <input
                  className="panel-input panel-input-full"
                  value={params.connection_string ?? ""}
                  onChange={(e) => updateParam("connection_string", e.target.value)}
                  placeholder={
                    isMongoDB
                      ? "mongodb+srv://user:pass@cluster.mongodb.net/db"
                      : selectedDriver === "supabase"
                      ? "postgresql://postgres:[pass]@db.[ref].supabase.co:5432/postgres"
                      : "postgresql://user:pass@host/db"
                  }
                />
              </label>
            )}

            {/* Turso */}
            {isTurso && (
              <>
                <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                  <span style={{ fontSize: 11, color: "var(--text-muted)", fontWeight: 600 }}>URL</span>
                  <input className="panel-input panel-input-full" value={params.url ?? ""} onChange={(e) => updateParam("url", e.target.value)} placeholder="libsql://your-db.turso.io" />
                </label>
                <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                  <span style={{ fontSize: 11, color: "var(--text-muted)", fontWeight: 600 }}>Auth token</span>
                  <input className="panel-input panel-input-full" type="password" value={params.token ?? ""} onChange={(e) => updateParam("token", e.target.value)} placeholder="eyJ..." />
                </label>
              </>
            )}

            {/* Elasticsearch / OpenSearch */}
            {isElastic && (
              <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                <span style={{ fontSize: 11, color: "var(--text-muted)", fontWeight: 600 }}>URL</span>
                <input className="panel-input panel-input-full" value={params.url ?? ""} onChange={(e) => updateParam("url", e.target.value)} placeholder="http://localhost:9200" />
              </label>
            )}

            {/* Snowflake */}
            {isSnowflake && (
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 }}>
                <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                  <span style={{ fontSize: 11, color: "var(--text-muted)", fontWeight: 600 }}>Account identifier</span>
                  <input className="panel-input panel-input-full" value={params.account ?? ""} onChange={(e) => updateParam("account", e.target.value)} placeholder="org-account" />
                </label>
                <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                  <span style={{ fontSize: 11, color: "var(--text-muted)", fontWeight: 600 }}>Warehouse</span>
                  <input className="panel-input panel-input-full" value={params.warehouse ?? ""} onChange={(e) => updateParam("warehouse", e.target.value)} placeholder="COMPUTE_WH" />
                </label>
                <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                  <span style={{ fontSize: 11, color: "var(--text-muted)", fontWeight: 600 }}>Username</span>
                  <input className="panel-input panel-input-full" value={params.username ?? ""} onChange={(e) => updateParam("username", e.target.value)} placeholder="user" />
                </label>
                <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                  <span style={{ fontSize: 11, color: "var(--text-muted)", fontWeight: 600 }}>Password</span>
                  <input className="panel-input panel-input-full" type="password" value={params.password ?? ""} onChange={(e) => updateParam("password", e.target.value)} placeholder="••••••••" />
                </label>
                <label style={{ display: "flex", flexDirection: "column", gap: 4, gridColumn: "1 / -1" }}>
                  <span style={{ fontSize: 11, color: "var(--text-muted)", fontWeight: 600 }}>Database</span>
                  <input className="panel-input panel-input-full" value={params.database ?? ""} onChange={(e) => updateParam("database", e.target.value)} placeholder="MY_DATABASE" />
                </label>
              </div>
            )}

            {/* BigQuery */}
            {isBigQuery && (
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 }}>
                <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                  <span style={{ fontSize: 11, color: "var(--text-muted)", fontWeight: 600 }}>Project ID</span>
                  <input className="panel-input panel-input-full" value={params.project ?? ""} onChange={(e) => updateParam("project", e.target.value)} placeholder="my-gcp-project" />
                </label>
                <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                  <span style={{ fontSize: 11, color: "var(--text-muted)", fontWeight: 600 }}>Dataset</span>
                  <input className="panel-input panel-input-full" value={params.dataset ?? ""} onChange={(e) => updateParam("dataset", e.target.value)} placeholder="my_dataset" />
                </label>
              </div>
            )}

            {/* KV (Redis/Valkey) */}
            {isKV && (
              <div style={{ display: "grid", gridTemplateColumns: "1fr auto", gap: 10 }}>
                <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                  <span style={{ fontSize: 11, color: "var(--text-muted)", fontWeight: 600 }}>Host</span>
                  <input className="panel-input panel-input-full" value={params.host ?? ""} onChange={(e) => updateParam("host", e.target.value)} placeholder="127.0.0.1" />
                </label>
                <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                  <span style={{ fontSize: 11, color: "var(--text-muted)", fontWeight: 600 }}>Port</span>
                  <input className="panel-input" style={{ width: 80 }} type="number" value={params.port ?? 6379} onChange={(e) => updateParam("port", Number(e.target.value))} />
                </label>
                <label style={{ display: "flex", flexDirection: "column", gap: 4, gridColumn: "1 / -1" }}>
                  <span style={{ fontSize: 11, color: "var(--text-muted)", fontWeight: 600 }}>Password (optional)</span>
                  <input className="panel-input panel-input-full" type="password" value={params.password ?? ""} onChange={(e) => updateParam("password", e.target.value)} placeholder="••••••••" />
                </label>
              </div>
            )}

            {/* Network-based (default) */}
            {!isFileBased && !isCloudString && !isElastic && !isSnowflake && !isBigQuery && !isKV && (
              <>
                <div style={{ display: "grid", gridTemplateColumns: "1fr auto", gap: 10 }}>
                  <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                    <span style={{ fontSize: 11, color: "var(--text-muted)", fontWeight: 600 }}>Host</span>
                    <input className="panel-input panel-input-full" value={params.host ?? ""} onChange={(e) => updateParam("host", e.target.value)} placeholder="localhost" />
                  </label>
                  <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                    <span style={{ fontSize: 11, color: "var(--text-muted)", fontWeight: 600 }}>Port</span>
                    <input className="panel-input" style={{ width: 80 }} type="number" value={params.port ?? ""} onChange={(e) => updateParam("port", Number(e.target.value))} />
                  </label>
                </div>
                <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                  <span style={{ fontSize: 11, color: "var(--text-muted)", fontWeight: 600 }}>Database</span>
                  <input className="panel-input panel-input-full" value={params.database ?? ""} onChange={(e) => updateParam("database", e.target.value)} placeholder="mydb" />
                </label>
                <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 }}>
                  <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                    <span style={{ fontSize: 11, color: "var(--text-muted)", fontWeight: 600 }}>Username</span>
                    <input className="panel-input panel-input-full" value={params.username ?? ""} onChange={(e) => updateParam("username", e.target.value)} placeholder="admin" />
                  </label>
                  <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                    <span style={{ fontSize: 11, color: "var(--text-muted)", fontWeight: 600 }}>Password</span>
                    <input className="panel-input panel-input-full" type="password" value={params.password ?? ""} onChange={(e) => updateParam("password", e.target.value)} placeholder="••••••••" />
                  </label>
                </div>
                <label style={{ display: "flex", alignItems: "center", gap: 8, cursor: "pointer" }}>
                  <input type="checkbox" checked={params.ssl ?? false} onChange={(e) => updateParam("ssl", e.target.checked)} />
                  <span style={{ fontSize: 12, color: "var(--text-primary)" }}>Use SSL / TLS</span>
                </label>
              </>
            )}

            {/* Test result banner */}
            {testStatus === "ok" && (
              <div style={{ background: "rgba(34,197,94,0.15)", border: "1px solid #22c55e", borderRadius: 6, padding: "8px 12px", fontSize: 12, color: "#22c55e" }}>
                ✓ {testMessage}
              </div>
            )}
            {testStatus === "fail" && (
              <div className="panel-error" role="alert" style={{ fontSize: 12, padding: "8px 12px" }}>
                {testMessage}
              </div>
            )}
          </div>
        )}

        {/* Footer */}
        {step === 2 && (
          <div
            style={{
              padding: "12px 20px",
              borderTop: "1px solid var(--border-color, #3a3a5c)",
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
              gap: 8,
            }}
          >
            <button onClick={() => setStep(1)} className="panel-btn panel-btn-secondary">
              ← Back
            </button>
            <div style={{ display: "flex", gap: 8 }}>
              <button
                onClick={handleTestConnection}
                disabled={testStatus === "testing"}
                className="panel-btn panel-btn-secondary"
              >
                {testStatus === "testing" ? (
                  <span style={{ display: "flex", alignItems: "center", gap: 6 }}>
                    <Loader2 size={12} style={{ animation: "spin 1s linear infinite" }} />
                    Testing…
                  </span>
                ) : (
                  "Test Connection"
                )}
              </button>
              <button
                onClick={handleSaveAndConnect}
                disabled={isSaving}
                className="panel-btn panel-btn-primary"
              >
                {isSaving ? (
                  <span style={{ display: "flex", alignItems: "center", gap: 6 }}>
                    <Loader2 size={12} style={{ animation: "spin 1s linear infinite" }} />
                    Saving…
                  </span>
                ) : (
                  "Save & Connect"
                )}
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

// ─── Schema Tree ──────────────────────────────────────────────────────────────

interface SchemaTreeProps {
  tables: TableInfo[];
  selectedTable: string | null;
  onTableClick: (name: string) => void;
}

function SchemaTree({ tables, selectedTable, onTableClick }: SchemaTreeProps) {
  const [expanded, setExpanded] = useState<Set<string>>(new Set());

  const toggle = (name: string) =>
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });

  if (tables.length === 0) {
    return (
      <div style={{ padding: 12, fontSize: 11, opacity: 0.5, textAlign: "center" }}>
        No schema loaded
      </div>
    );
  }

  return (
    <div style={{ padding: "4px 0" }}>
      {tables.map((tbl) => {
        const isExpanded = expanded.has(tbl.name);
        const isSelected = selectedTable === tbl.name;
        return (
          <div key={tbl.name}>
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: 4,
                padding: "4px 10px",
                background: isSelected ? "var(--bg-tertiary)" : "transparent",
                cursor: "pointer",
                fontSize: 12,
                color: "var(--text-primary)",
                userSelect: "none",
              }}
              onClick={() => onTableClick(tbl.name)}
              onDoubleClick={() => toggle(tbl.name)}
            >
              <span
                style={{ fontSize: 9, opacity: 0.6, width: 10, flexShrink: 0 }}
                onClick={(ev) => { ev.stopPropagation(); toggle(tbl.name); }}
              >
                {isExpanded ? "▼" : "▶"}
              </span>
              <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", flex: 1 }}>
                📋 {tbl.name}
              </span>
              <span style={{ fontSize: 9, opacity: 0.4, flexShrink: 0 }}>
                {tbl.row_count.toLocaleString()}
              </span>
            </div>
            {isExpanded && (
              <div style={{ paddingLeft: 22 }}>
                {tbl.columns.map((col) => (
                  <div
                    key={col.name}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: 4,
                      padding: "2px 8px",
                      fontSize: 11,
                      color: "var(--text-muted)",
                      fontFamily: "var(--font-mono)",
                    }}
                  >
                    {col.primary_key && <span style={{ color: "#f59e0b", fontSize: 9 }}>PK</span>}
                    <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", flex: 1 }}>
                      {col.name}
                    </span>
                    <span style={{ fontSize: 10, opacity: 0.45, flexShrink: 0 }}>{col.data_type}</span>
                    {col.nullable && <span style={{ fontSize: 9, opacity: 0.35 }}>?</span>}
                  </div>
                ))}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

// ─── Main Panel ───────────────────────────────────────────────────────────────

const MAX_HISTORY = 20;

export function DatabasePanel({ workspacePath, provider }: DatabasePanelProps) {
  const { toasts, toast, dismiss } = useToast();

  // Saved profiles
  const [profiles, setProfiles] = useState<DbSavedProfile[]>([]);
  const [profilesLoaded, setProfilesLoaded] = useState(false);

  // Active connection
  const [activeConn, setActiveConn] = useState<ActiveConnection | null>(null);

  // Selected table
  const [selectedTable, setSelectedTable] = useState<string | null>(null);

  // Query state
  const [nlQuery, setNlQuery] = useState("");
  const [sqlQuery, setSqlQuery] = useState("");
  const [queryResult, setQueryResult] = useState<QueryResult | null>(null);
  const [isQuerying, setIsQuerying] = useState(false);
  const [queryHistory, setQueryHistory] = useState<string[]>([]);
  const [showHistory, setShowHistory] = useState(false);

  // UI state
  const [showWizard, setShowWizard] = useState(false);
  const [isNlLoading, setIsNlLoading] = useState(false);

  const sqlTextareaRef = useRef<HTMLTextAreaElement>(null);

  // Load saved profiles
  useEffect(() => {
    if (!workspacePath) return;
    invoke<DbSavedProfile[]>("db_list_profiles", { workspacePath })
      .then((p) => { setProfiles(p); setProfilesLoaded(true); })
      .catch(() => setProfilesLoaded(true));
  }, [workspacePath]);

  const connectToProfile = useCallback(async (profile: DbSavedProfile) => {
    setActiveConn({ profile, status: "connecting", tables: [] });
    setSelectedTable(null);
    setQueryResult(null);
    setSqlQuery("");
    try {
      const tables = await invoke<TableInfo[]>("db_schema", { params: profile.params });
      setActiveConn({ profile, status: "connected", tables });
      toast.success(`Connected: ${profile.name}`);
    } catch (e) {
      setActiveConn({ profile, status: "error", tables: [] });
      toast.error(`Connection failed: ${e}`);
    }
  }, [toast]);

  const handleProfileAdded = useCallback((profile: DbSavedProfile) => {
    setProfiles((prev) => [...prev, profile]);
    connectToProfile(profile);
  }, [connectToProfile]);

  const handleDeleteProfile = useCallback(async (id: string) => {
    if (!workspacePath) return;
    try {
      await invoke<void>("db_delete_profile", { workspacePath, profileId: id });
      setProfiles((prev) => prev.filter((p) => p.id !== id));
      if (activeConn?.profile.id === id) {
        setActiveConn(null);
        setSelectedTable(null);
        setQueryResult(null);
      }
      toast.info("Connection removed.");
    } catch (e) {
      toast.error(`Delete failed: ${e}`);
    }
  }, [workspacePath, activeConn, toast]);

  const runQuery = useCallback(async (sql: string, params?: DbConnectionParams) => {
    const connParams = params ?? activeConn?.profile.params;
    if (!connParams) { toast.warn("No active connection."); return; }
    const trimmed = sql.trim();
    if (!trimmed) return;
    setIsQuerying(true);
    setQueryResult(null);
    try {
      const result = await invoke<QueryResult>("db_query", { params: connParams, sql: trimmed });
      setQueryResult(result);
      if (result.error) {
        toast.error(`Query error: ${result.error}`);
      }
      setQueryHistory((prev) => [trimmed, ...prev.filter((q) => q !== trimmed)].slice(0, MAX_HISTORY));
    } catch (e) {
      setQueryResult({ columns: [], rows: [], row_count: 0, error: String(e) });
      toast.error(`Query failed: ${e}`);
    } finally {
      setIsQuerying(false);
    }
  }, [activeConn, toast]);

  const handleTableClick = useCallback(async (tableName: string) => {
    setSelectedTable(tableName);
    const sql = `SELECT * FROM "${tableName}" LIMIT 100`;
    setSqlQuery(sql);
    await runQuery(sql);
  }, [runQuery]);

  const handleNlQuery = useCallback(async () => {
    if (!nlQuery.trim() || !activeConn) return;
    setIsNlLoading(true);
    try {
      const schema = activeConn.tables
        .map(
          (t) =>
            `${t.name}(${t.columns
              .map((c) => `${c.name} ${c.data_type}${c.primary_key ? " PK" : ""}`)
              .join(", ")})`
        )
        .join("\n");
      const sql = await invoke<string>("generate_sql_query", {
        description: nlQuery,
        schema,
        provider,
      });
      setSqlQuery(sql);
      await runQuery(sql);
    } catch (e) {
      toast.error(`AI query failed: ${e}`);
    } finally {
      setIsNlLoading(false);
    }
  }, [nlQuery, activeConn, provider, runQuery, toast]);

  const handleGenerateMigration = useCallback(async () => {
    if (!activeConn) { toast.warn("No active connection."); return; }
    const desc = prompt("Describe the migration (e.g., 'Add email column to users table'):");
    if (!desc) return;
    setIsQuerying(true);
    try {
      const migration = await invoke<string>("generate_migration", {
        params: activeConn.profile.params,
        description: desc,
        provider,
      });
      setSqlQuery(migration);
      toast.info("Migration script generated.");
    } catch (e) {
      toast.error(`Migration failed: ${e}`);
    } finally {
      setIsQuerying(false);
    }
  }, [activeConn, provider, toast]);

  const handleExportCSV = useCallback(() => {
    if (!queryResult || queryResult.rows.length === 0) return;
    const csv = toCSV(queryResult.columns, queryResult.rows);
    exportBlob(csv, "query-result.csv", "text/csv");
    toast.success("CSV exported.");
  }, [queryResult, toast]);

  const handleExportJSON = useCallback(() => {
    if (!queryResult || queryResult.rows.length === 0) return;
    const json = JSON.stringify(queryResult.rows, null, 2);
    exportBlob(json, "query-result.json", "application/json");
    toast.success("JSON exported.");
  }, [queryResult, toast]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        runQuery(sqlQuery);
      }
    },
    [runQuery, sqlQuery]
  );

  // ── No workspace ────────────────────────────────────────────────────────────
  if (!workspacePath) {
    return (
      <div className="panel-container" style={{ alignItems: "center", justifyContent: "center" }}>
        <div style={{ textAlign: "center", opacity: 0.6, fontSize: 13 }}>
          <div style={{ fontSize: 32, marginBottom: 12 }}>🗄️</div>
          <p>Open a workspace folder to use the database browser.</p>
        </div>
      </div>
    );
  }

  const connEntry = activeConn ? catalogEntry(activeConn.profile.driver) : null;

  return (
    <div className="panel-container" style={{ flexDirection: "row", position: "relative", overflow: "hidden" }}>

      {/* ── Left: Saved connections (200px) ─────────────────────────────── */}
      <div
        style={{
          width: 210,
          flexShrink: 0,
          borderRight: "1px solid var(--border-color)",
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
        }}
      >
        <div
          style={{
            padding: "10px 12px 8px",
            borderBottom: "1px solid var(--border-color)",
            fontSize: 11,
            fontWeight: 700,
            textTransform: "uppercase",
            letterSpacing: "0.07em",
            color: "var(--text-muted)",
          }}
        >
          Connections
        </div>

        <div style={{ flex: 1, overflowY: "auto", padding: "6px 4px" }}>
          {!profilesLoaded && (
            <div style={{ padding: 12, display: "flex", justifyContent: "center" }}>
              <Loader2 size={14} style={{ animation: "spin 1s linear infinite", opacity: 0.5 }} />
            </div>
          )}
          {profilesLoaded && profiles.length === 0 && (
            <div style={{ padding: "16px 12px", fontSize: 11, opacity: 0.5, textAlign: "center" }}>
              No saved connections
            </div>
          )}
          {profiles.map((p) => {
            const entry = catalogEntry(p.driver);
            const isActive = activeConn?.profile.id === p.id;
            const status: ConnectionStatus = isActive ? (activeConn?.status ?? "disconnected") : "disconnected";
            return (
              <div
                key={p.id}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 6,
                  padding: "6px 8px",
                  borderRadius: 5,
                  background: isActive ? "var(--bg-tertiary)" : "transparent",
                  cursor: "pointer",
                  marginBottom: 2,
                }}
                onClick={() => connectToProfile(p)}
                title={p.name}
              >
                <span style={{ fontSize: 14, flexShrink: 0 }}>{entry.icon}</span>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div
                    style={{
                      fontSize: 12,
                      fontWeight: isActive ? 600 : 400,
                      color: "var(--text-primary)",
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap",
                    }}
                  >
                    {p.name}
                  </div>
                  <div style={{ fontSize: 10, opacity: 0.5 }}>{entry.label}</div>
                </div>
                <StatusDot status={status} />
                <button
                  aria-label={`Delete ${p.name}`}
                  onClick={(ev) => { ev.stopPropagation(); handleDeleteProfile(p.id); }}
                  style={{
                    background: "none",
                    border: "none",
                    color: "var(--text-muted)",
                    cursor: "pointer",
                    fontSize: 14,
                    padding: 0,
                    lineHeight: 1,
                    opacity: 0,
                    transition: "opacity 0.15s",
                  }}
                  onMouseEnter={(ev) => { (ev.currentTarget as HTMLButtonElement).style.opacity = "1"; }}
                  onMouseLeave={(ev) => { (ev.currentTarget as HTMLButtonElement).style.opacity = "0"; }}
                >
                  ×
                </button>
              </div>
            );
          })}
        </div>

        {/* + New button */}
        <div style={{ padding: 8, borderTop: "1px solid var(--border-color)" }}>
          <button
            onClick={() => setShowWizard(true)}
            className="panel-btn panel-btn-primary"
            style={{ width: "100%", fontSize: 12 }}
          >
            ＋ New Connection
          </button>
        </div>
      </div>

      {/* ── Center: Schema tree (200px) ──────────────────────────────────── */}
      <div
        style={{
          width: 210,
          flexShrink: 0,
          borderRight: "1px solid var(--border-color)",
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
        }}
      >
        <div
          style={{
            padding: "10px 12px 8px",
            borderBottom: "1px solid var(--border-color)",
            display: "flex",
            alignItems: "center",
            gap: 6,
          }}
        >
          {connEntry && <span style={{ fontSize: 13 }}>{connEntry.icon}</span>}
          <span
            style={{
              fontSize: 11,
              fontWeight: 700,
              textTransform: "uppercase",
              letterSpacing: "0.07em",
              color: "var(--text-muted)",
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
              flex: 1,
            }}
          >
            {activeConn ? activeConn.profile.name : "Schema"}
          </span>
          {activeConn && <StatusDot status={activeConn.status} />}
        </div>

        <div style={{ flex: 1, overflowY: "auto" }}>
          {activeConn?.status === "connecting" && (
            <div style={{ padding: 16, display: "flex", justifyContent: "center" }}>
              <Loader2 size={16} style={{ animation: "spin 1s linear infinite", opacity: 0.6 }} />
            </div>
          )}
          {activeConn?.status === "error" && (
            <div className="panel-error" style={{ margin: 8, fontSize: 11, padding: "6px 10px" }} role="alert">
              Connection error. Click to retry.
            </div>
          )}
          {(activeConn?.status === "connected") && (
            <SchemaTree
              tables={activeConn.tables}
              selectedTable={selectedTable}
              onTableClick={handleTableClick}
            />
          )}
          {!activeConn && (
            <div style={{ padding: "20px 12px", fontSize: 11, opacity: 0.4, textAlign: "center" }}>
              Select a connection to view schema
            </div>
          )}
        </div>
      </div>

      {/* ── Right: Query + Results (flex-1) ──────────────────────────────── */}
      <div style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column", overflow: "hidden" }}>

        {/* NL query bar */}
        <div
          style={{
            padding: "10px 12px 8px",
            borderBottom: "1px solid var(--border-color)",
            display: "flex",
            gap: 8,
            alignItems: "center",
          }}
        >
          <input
            value={nlQuery}
            onChange={(e) => setNlQuery(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && !isNlLoading && handleNlQuery()}
            placeholder="Ask in plain English — e.g., 'Show users signed up this week'"
            className="panel-input panel-input-full"
            style={{ flex: 1, fontSize: 12 }}
            disabled={!activeConn || activeConn.status !== "connected"}
          />
          <button
            onClick={handleNlQuery}
            disabled={isNlLoading || !nlQuery.trim() || activeConn?.status !== "connected"}
            className="panel-btn panel-btn-primary"
            style={{ whiteSpace: "nowrap", fontSize: 12 }}
          >
            {isNlLoading ? (
              <span style={{ display: "flex", alignItems: "center", gap: 5 }}>
                <Loader2 size={11} style={{ animation: "spin 1s linear infinite" }} /> Thinking…
              </span>
            ) : (
              "✨ Ask AI"
            )}
          </button>
        </div>

        {/* SQL editor */}
        <div
          style={{
            padding: "8px 12px",
            borderBottom: "1px solid var(--border-color)",
            display: "flex",
            flexDirection: "column",
            gap: 6,
          }}
        >
          <div style={{ position: "relative" }}>
            <textarea
              ref={sqlTextareaRef}
              value={sqlQuery}
              onChange={(e) => setSqlQuery(e.target.value)}
              onKeyDown={handleKeyDown}
              rows={4}
              placeholder="SELECT * FROM users LIMIT 50   — Ctrl+Enter to run"
              className="panel-input panel-textarea panel-input-full"
              style={{ fontFamily: "var(--font-mono)", fontSize: 12, resize: "vertical", minHeight: 72 }}
              disabled={activeConn?.status !== "connected"}
            />
          </div>

          {/* Toolbar */}
          <div style={{ display: "flex", alignItems: "center", gap: 6, flexWrap: "wrap" }}>
            <button
              onClick={() => runQuery(sqlQuery)}
              disabled={isQuerying || !sqlQuery.trim() || activeConn?.status !== "connected"}
              className="panel-btn panel-btn-primary"
              style={{ fontSize: 12 }}
            >
              {isQuerying ? (
                <span style={{ display: "flex", alignItems: "center", gap: 5 }}>
                  <Loader2 size={11} style={{ animation: "spin 1s linear infinite" }} /> Running…
                </span>
              ) : (
                "▶ Run"
              )}
            </button>

            {/* History dropdown */}
            <div style={{ position: "relative" }}>
              <button
                onClick={() => setShowHistory((v) => !v)}
                className="panel-btn panel-btn-secondary"
                style={{ fontSize: 12 }}
                disabled={queryHistory.length === 0}
                title="Query history"
              >
                🕐 History
              </button>
              {showHistory && queryHistory.length > 0 && (
                <div
                  style={{
                    position: "absolute",
                    top: "100%",
                    left: 0,
                    zIndex: 20,
                    background: "var(--bg-primary)",
                    border: "1px solid var(--border-color)",
                    borderRadius: 6,
                    minWidth: 320,
                    maxHeight: 240,
                    overflowY: "auto",
                    boxShadow: "0 4px 16px rgba(0,0,0,0.4)",
                    marginTop: 2,
                  }}
                >
                  {queryHistory.map((q, i) => (
                    <button
                      key={i}
                      onClick={() => { setSqlQuery(q); setShowHistory(false); }}
                      style={{
                        display: "block",
                        width: "100%",
                        textAlign: "left",
                        background: "none",
                        border: "none",
                        padding: "7px 12px",
                        fontSize: 11,
                        fontFamily: "var(--font-mono)",
                        color: "var(--text-primary)",
                        cursor: "pointer",
                        borderBottom: i < queryHistory.length - 1 ? "1px solid var(--border-color)" : "none",
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                      }}
                      onMouseEnter={(ev) => { (ev.currentTarget as HTMLButtonElement).style.background = "var(--bg-secondary)"; }}
                      onMouseLeave={(ev) => { (ev.currentTarget as HTMLButtonElement).style.background = "none"; }}
                    >
                      {q}
                    </button>
                  ))}
                </div>
              )}
            </div>

            <div style={{ marginLeft: "auto", display: "flex", gap: 6 }}>
              <button
                onClick={handleExportCSV}
                disabled={!queryResult || queryResult.rows.length === 0}
                className="panel-btn panel-btn-secondary"
                style={{ fontSize: 12 }}
                title="Export as CSV"
              >
                ⬇ CSV
              </button>
              <button
                onClick={handleExportJSON}
                disabled={!queryResult || queryResult.rows.length === 0}
                className="panel-btn panel-btn-secondary"
                style={{ fontSize: 12 }}
                title="Export as JSON"
              >
                ⬇ JSON
              </button>
              <button
                onClick={handleGenerateMigration}
                disabled={isQuerying || activeConn?.status !== "connected"}
                className="panel-btn panel-btn-secondary"
                style={{ fontSize: 12 }}
                title="Generate migration script"
              >
                ＋ Migration
              </button>
            </div>
          </div>
        </div>

        {/* Results */}
        <div style={{ flex: 1, overflow: "auto", padding: 12 }}>
          {/* Empty state */}
          {!activeConn && (
            <div
              style={{
                display: "flex",
                flexDirection: "column",
                alignItems: "center",
                justifyContent: "center",
                height: "100%",
                opacity: 0.5,
                gap: 12,
              }}
            >
              <div style={{ fontSize: 40 }}>🗄️</div>
              <div style={{ fontSize: 13 }}>Select a connection or create a new one to get started.</div>
              <button
                onClick={() => setShowWizard(true)}
                className="panel-btn panel-btn-primary"
              >
                ＋ New Connection
              </button>
            </div>
          )}

          {/* Loading */}
          {isQuerying && (
            <div style={{ display: "flex", alignItems: "center", gap: 8, color: "var(--text-muted)", fontSize: 12 }}>
              <Loader2 size={14} style={{ animation: "spin 1s linear infinite" }} /> Running query…
            </div>
          )}

          {/* Error */}
          {queryResult?.error && !isQuerying && (
            <div className="panel-error" role="alert" style={{ fontFamily: "var(--font-mono)", fontSize: 12, padding: "8px 12px", whiteSpace: "pre-wrap" }}>
              {queryResult.error}
            </div>
          )}

          {/* Results table */}
          {queryResult && !queryResult.error && !isQuerying && (
            <>
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 10,
                  marginBottom: 8,
                  fontSize: 11,
                  color: "var(--text-muted)",
                }}
              >
                <span
                  style={{
                    background: "var(--bg-secondary)",
                    padding: "2px 8px",
                    borderRadius: 10,
                    fontWeight: 600,
                    color: "var(--text-primary)",
                  }}
                >
                  {queryResult.row_count.toLocaleString()} row{queryResult.row_count !== 1 ? "s" : ""}
                </span>
                {queryResult.rows.length === 0 && <span>No rows returned.</span>}
              </div>

              {queryResult.rows.length > 0 && (
                <div style={{ overflowX: "auto" }}>
                  <table
                    style={{
                      width: "100%",
                      borderCollapse: "collapse",
                      fontSize: 12,
                      fontFamily: "var(--font-mono)",
                    }}
                  >
                    <thead>
                      <tr style={{ background: "var(--bg-secondary)" }}>
                        <th
                          scope="col"
                          style={{
                            padding: "4px 8px",
                            textAlign: "right",
                            borderBottom: "1px solid var(--border-color)",
                            fontWeight: 500,
                            color: "var(--text-muted)",
                            fontSize: 10,
                            width: 36,
                          }}
                        >
                          #
                        </th>
                        {queryResult.columns.map((col) => (
                          <th
                            key={col}
                            scope="col"
                            style={{
                              padding: "4px 10px",
                              textAlign: "left",
                              borderBottom: "1px solid var(--border-color)",
                              fontWeight: 600,
                              whiteSpace: "nowrap",
                              color: "var(--text-primary)",
                            }}
                          >
                            {col}
                          </th>
                        ))}
                      </tr>
                    </thead>
                    <tbody>
                      {queryResult.rows.map((row, i) => (
                        <tr
                          key={i}
                          style={{ background: i % 2 === 0 ? "transparent" : "var(--bg-secondary)" }}
                        >
                          <td
                            style={{
                              padding: "3px 8px",
                              borderBottom: "1px solid var(--border-color)",
                              color: "var(--text-muted)",
                              fontSize: 10,
                              textAlign: "right",
                              userSelect: "none",
                            }}
                          >
                            {i + 1}
                          </td>
                          {queryResult.columns.map((col) => {
                            const val = row[col];
                            const isNull = val === null || val === undefined;
                            return (
                              <td
                                key={col}
                                style={{
                                  padding: "3px 10px",
                                  borderBottom: "1px solid var(--border-color)",
                                  opacity: isNull ? 0.35 : 1,
                                  maxWidth: 240,
                                  overflow: "hidden",
                                  textOverflow: "ellipsis",
                                  whiteSpace: "nowrap",
                                  fontStyle: isNull ? "italic" : "normal",
                                  color: isNull ? "var(--text-muted)" : "var(--text-primary)",
                                }}
                                title={isNull ? "NULL" : String(val)}
                              >
                                {isNull ? "NULL" : String(val)}
                              </td>
                            );
                          })}
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </>
          )}
        </div>
      </div>

      {/* ── Connection Wizard (overlay) ───────────────────────────────────── */}
      {showWizard && (
        <ConnectionWizard
          workspacePath={workspacePath}
          onClose={() => setShowWizard(false)}
          onSaved={handleProfileAdded}
          toast={toast}
        />
      )}

      <Toaster toasts={toasts} onDismiss={dismiss} />
    </div>
  );
}

export default DatabasePanel;
