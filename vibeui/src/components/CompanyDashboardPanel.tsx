/**
 * CompanyDashboardPanel — Real-time company status overview.
 *
 * Shows active company info, agent count, recent activity feed,
 * and quick action buttons. Uses company_status Tauri command.
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface CompanyDashboardPanelProps {
  workspacePath?: string | null;
}

export function CompanyDashboardPanel({ workspacePath: _wp }: CompanyDashboardPanelProps) {
  const [statusText, setStatusText] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [cmd, setCmd] = useState("");
  const [cmdOutput, setCmdOutput] = useState<string | null>(null);
  const [cmdLoading, setCmdLoading] = useState(false);

  const loadStatus = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const out = await invoke<string>("company_status");
      setStatusText(out);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { loadStatus(); }, [loadStatus]);

  const runCmd = async () => {
    if (!cmd.trim()) return;
    setCmdLoading(true);
    setCmdOutput(null);
    try {
      const out = await invoke<string>("company_cmd", { args: cmd.trim() });
      setCmdOutput(out);
      // Refresh status after mutation
      loadStatus();
    } catch (e) {
      setCmdOutput(`Error: ${e}`);
    } finally {
      setCmdLoading(false);
    }
  };

  const quickActions = [
    { label: "List Companies", args: "list" },
    { label: "List Agents", args: "agent list" },
    { label: "Org Chart", args: "agent tree" },
    { label: "Activity Log", args: "status" },
  ];

  return (
    <div style={{ padding: 16, height: "100%", overflowY: "auto", fontSize: 13 }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 16 }}>
        <span style={{ fontWeight: 700, fontSize: 15 }}>Company Dashboard</span>
        <button onClick={loadStatus} style={{ fontSize: 11, padding: "2px 8px", cursor: "pointer" }}>
          Refresh
        </button>
      </div>

      {/* Status panel */}
      <div
        style={{
          background: "var(--panel-bg, rgba(0,0,0,0.2))",
          border: "1px solid var(--border)",
          borderRadius: 6,
          padding: 12,
          marginBottom: 16,
          minHeight: 120,
        }}
      >
        {loading && <span style={{ color: "var(--text-secondary)" }}>Loading…</span>}
        {error && <span style={{ color: "var(--danger, #e74c3c)" }}>{error}</span>}
        {!loading && !error && (
          <pre style={{ margin: 0, fontSize: 12, whiteSpace: "pre-wrap", lineHeight: 1.6 }}>
            {statusText || "No active company. Use: /company create <name>"}
          </pre>
        )}
      </div>

      {/* Quick actions */}
      <div style={{ marginBottom: 16 }}>
        <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 8 }}>Quick Actions</div>
        <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
          {quickActions.map((a) => (
            <button
              key={a.args}
              onClick={async () => {
                setCmdOutput(null);
                setCmdLoading(true);
                try {
                  const out = await invoke<string>("company_cmd", { args: a.args });
                  setCmdOutput(out);
                } catch (e) {
                  setCmdOutput(`Error: ${e}`);
                } finally {
                  setCmdLoading(false);
                }
              }}
              style={{ fontSize: 11, padding: "3px 10px", cursor: "pointer", borderRadius: 4 }}
            >
              {a.label}
            </button>
          ))}
        </div>
      </div>

      {/* Command input */}
      <div style={{ marginBottom: 16 }}>
        <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 6 }}>
          Run Company Command
        </div>
        <div style={{ display: "flex", gap: 6 }}>
          <input
            value={cmd}
            onChange={(e) => setCmd(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && runCmd()}
            placeholder="agent hire Alice --title CEO --role ceo"
            style={{
              flex: 1,
              fontSize: 12,
              padding: "4px 8px",
              background: "var(--input-bg, rgba(0,0,0,0.3))",
              border: "1px solid var(--border)",
              borderRadius: 4,
              color: "var(--text-primary)",
            }}
          />
          <button
            onClick={runCmd}
            disabled={cmdLoading}
            style={{ fontSize: 11, padding: "4px 12px", cursor: "pointer" }}
          >
            {cmdLoading ? "…" : "Run"}
          </button>
        </div>
      </div>

      {/* Command output */}
      {cmdOutput !== null && (
        <div
          style={{
            background: "var(--panel-bg, rgba(0,0,0,0.2))",
            border: "1px solid var(--border)",
            borderRadius: 6,
            padding: 12,
          }}
        >
          <pre style={{ margin: 0, fontSize: 12, whiteSpace: "pre-wrap", lineHeight: 1.5 }}>
            {cmdOutput}
          </pre>
        </div>
      )}
    </div>
  );
}
