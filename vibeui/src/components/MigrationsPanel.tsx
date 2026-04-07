/**
 * MigrationsPanel — Database Migration Manager.
 *
 * Auto-detects migration tool (Prisma, Diesel, Alembic, Flyway, golang-migrate),
 * lists pending/applied migrations, and provides run/rollback/generate actions.
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface MigrationStatus {
 tool: string;
 applied: MigrationEntry[];
 pending: MigrationEntry[];
 raw_output: string;
}

interface MigrationEntry {
 name: string;
 applied_at: string | null;
 state: "applied" | "pending" | "failed";
}

interface MigrationsPanelProps {
 workspacePath: string | null;
}

const TOOL_ICONS: Record<string, string> = {
 prisma: "",
 diesel: "",
 alembic: "",
 flyway: "",
 "golang-migrate": "",
 unknown: "",
};

const STATE_COLORS: Record<string, string> = {
 applied: "var(--success-color)",
 pending: "var(--warning-color)",
 failed: "var(--error-color)",
};

const STATE_ICONS: Record<string, string> = {
 applied: "",
 pending: "",
 failed: "",
};

export function MigrationsPanel({ workspacePath }: MigrationsPanelProps) {
 const [status, setStatus] = useState<MigrationStatus | null>(null);
 const [loading, setLoading] = useState(false);
 const [actionLoading, setActionLoading] = useState<string | null>(null);
 const [output, setOutput] = useState<string | null>(null);
 const [error, setError] = useState<string | null>(null);
 const [newMigName, setNewMigName] = useState("");
 const [showRaw, setShowRaw] = useState(false);

 const load = async () => {
 if (!workspacePath) return;
 setLoading(true);
 setError(null);
 setOutput(null);
 try {
 const result = await invoke<MigrationStatus>("get_migration_status", {
 workspace: workspacePath,
 });
 setStatus(result);
 } catch (e) {
 setError(String(e));
 } finally {
 setLoading(false);
 }
 };

 useEffect(() => {
 load();
 }, [workspacePath]);

 const runAction = async (action: string, extra?: string) => {
 if (!workspacePath || !status) return;
 setActionLoading(action);
 setOutput(null);
 setError(null);
 try {
 const out = await invoke<string>("run_migration_action", {
 workspace: workspacePath,
 tool: status.tool,
 action,
 extra: extra ?? null,
 });
 setOutput(out);
 await load();
 } catch (e) {
 setError(String(e));
 } finally {
 setActionLoading(null);
 }
 };

 const handleGenerate = async () => {
 const name = newMigName.trim();
 if (!name) return;
 await runAction("generate", name);
 setNewMigName("");
 };

 if (!workspacePath) {
 return (
 <div style={{ padding: 24, textAlign: "center", color: "var(--text-secondary)", fontSize: 12 }}>
 Open a workspace folder to manage migrations.
 </div>
 );
 }

 const tool = status?.tool ?? "unknown";
 const allMigrations = [...(status?.applied ?? []), ...(status?.pending ?? [])]
 .sort((a, b) => a.name.localeCompare(b.name));

 return (
 <div className="panel-container">
 {/* Header */}
 <div className="panel-header">
 <span style={{ fontSize: 16 }}>{TOOL_ICONS[tool]}</span>
 <h3>
 {tool === "unknown" ? "No migration tool detected" : tool.charAt(0).toUpperCase() + tool.slice(1)}
 </h3>
 {status && (
 <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>
 {status.applied.length} applied · {status.pending.length} pending
 </span>
 )}
 <button
 onClick={load}
 disabled={loading}
 className="panel-btn panel-btn-secondary"
 style={{ marginLeft: "auto" }}
 >
 {loading ? "" : "↻ Refresh"}
 </button>
 </div>

 <div className="panel-body" style={{ display: "flex", flexDirection: "column", gap: 10 }}>
 {error && (
 <div className="panel-error" style={{ padding: "7px 10px", fontSize: 12 }}>
 {error}
 </div>
 )}

 {/* Actions */}
 {status && tool !== "unknown" && (
 <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
 <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
 <button
 onClick={() => runAction("migrate")}
 disabled={!!actionLoading || status.pending.length === 0}
 style={{
 padding: "6px 12px", fontSize: 12, fontWeight: 600,
 background: status.pending.length > 0 ? "var(--accent-color)" : "var(--bg-secondary)",
 color: status.pending.length > 0 ? "var(--text-primary)" : "var(--text-secondary)",
 border: "none", borderRadius: 4, cursor: status.pending.length > 0 ? "pointer" : "default",
 opacity: actionLoading === "migrate" ? 0.7 : 1,
 }}
 >
 {actionLoading === "migrate" ? "Running…" : ` Migrate (${status.pending.length} pending)`}
 </button>
 <button
 onClick={() => { if (confirm("Rollback the last migration?")) runAction("rollback"); }}
 disabled={!!actionLoading || status.applied.length === 0}
 style={{
 padding: "6px 12px", fontSize: 12,
 background: "color-mix(in srgb, var(--accent-rose) 15%, transparent)", color: "var(--error-color)",
 border: "1px solid var(--error-color)", borderRadius: 4,
 cursor: status.applied.length > 0 ? "pointer" : "default",
 opacity: actionLoading === "rollback" ? 0.7 : 1,
 }}
 >
 {actionLoading === "rollback" ? "Rolling back…" : "Rollback"}
 </button>
 <button
 onClick={() => runAction("status")}
 disabled={!!actionLoading}
 style={{ padding: "6px 12px", fontSize: 12, background: "var(--bg-secondary)", color: "var(--text-secondary)", border: "1px solid var(--border-color)", borderRadius: 4, cursor: "pointer" }}
 >
 {actionLoading === "status" ? "" : "Status"}
 </button>
 </div>

 {/* Generate new migration */}
 <div style={{ display: "flex", gap: 6 }}>
 <input
 value={newMigName}
 onChange={(e) => setNewMigName(e.target.value)}
 onKeyDown={(e) => e.key === "Enter" && handleGenerate()}
 placeholder="Migration name (e.g. add_users_table)"
 style={{
 flex: 1, padding: "5px 8px", fontSize: 12,
 background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
 borderRadius: 4, color: "var(--text-primary)", outline: "none",
 }}
 />
 <button
 onClick={handleGenerate}
 disabled={!!actionLoading || !newMigName.trim()}
 style={{ padding: "5px 12px", fontSize: 12, background: "var(--bg-secondary)", color: "var(--text-secondary)", border: "1px solid var(--border-color)", borderRadius: 4, cursor: "pointer" }}
 >
 {actionLoading === "generate" ? "" : "+ Generate"}
 </button>
 </div>
 </div>
 )}

 {/* Output */}
 {output && (
 <pre style={{
 margin: 0, padding: 10, background: "var(--bg-primary)", color: "var(--text-primary)",
 border: "1px solid var(--border-color)", borderRadius: 6,
 fontSize: 11, lineHeight: 1.4, overflow: "auto", maxHeight: 180,
 whiteSpace: "pre-wrap", wordBreak: "break-all",
 }}>
 {output}
 </pre>
 )}

 {/* Migration list */}
 {allMigrations.length > 0 ? (
 <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
 <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 2, fontWeight: 600 }}>
 MIGRATIONS ({allMigrations.length})
 </div>
 {allMigrations.map((m) => (
 <div
 key={m.name}
 style={{
 display: "flex", alignItems: "center", gap: 10,
 padding: "7px 10px", borderRadius: 5,
 background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
 fontSize: 12,
 }}
 >
 <span>{STATE_ICONS[m.state]}</span>
 <span style={{ fontFamily: "var(--font-mono)", flex: 1, fontSize: 11 }}>{m.name}</span>
 <span style={{ fontSize: 10, color: STATE_COLORS[m.state], fontWeight: 600 }}>
 {m.state}
 </span>
 {m.applied_at && (
 <span style={{ fontSize: 10, color: "var(--text-secondary)" }}>
 {new Date(m.applied_at).toLocaleDateString()}
 </span>
 )}
 </div>
 ))}
 </div>
 ) : !loading && status && (
 <div className="panel-empty">
 {tool === "unknown"
 ? "No migration tool found. Add Prisma, Diesel, Alembic, Flyway, or golang-migrate."
 : "No migrations found. Generate one above."}
 </div>
 )}

 {/* Raw output toggle */}
 {status?.raw_output && (
 <div>
 <button
 onClick={() => setShowRaw((p) => !p)}
 style={{ fontSize: 11, background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer", padding: 0, marginBottom: 4 }}
 >
 {showRaw ? "Hide raw output" : "▼ Show raw output"}
 </button>
 {showRaw && (
 <pre style={{
 margin: 0, padding: 8, background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
 borderRadius: 4, fontSize: 10, lineHeight: 1.4,
 overflow: "auto", maxHeight: 200, whiteSpace: "pre-wrap", color: "var(--text-secondary)",
 }}>
 {status.raw_output}
 </pre>
 )}
 </div>
 )}
 </div>
 </div>
 );
}
