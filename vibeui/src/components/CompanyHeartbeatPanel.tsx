/**
 * CompanyHeartbeatPanel — Agent heartbeat run history and manual triggers.
 *
 * Shows company-wide or per-agent heartbeat runs with status badges,
 * trigger type, duration, and summary. Supports manual trigger.
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Heart, X, Check } from "lucide-react";

interface CompanyHeartbeatPanelProps {
  workspacePath?: string | null;
}

const btnStyle: React.CSSProperties = {
  fontSize: 11, padding: "3px 10px", cursor: "pointer", borderRadius: 4,
  background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", color: "var(--text-primary)",
};

const inputStyle: React.CSSProperties = {
  fontSize: 12, padding: "4px 8px", background: "var(--bg-primary)",
  border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)",
};

function statusColor(line: string): string {
  if (line.startsWith("✓")) return "var(--success, #27ae60)";
  if (line.startsWith("✗")) return "var(--danger, #e74c3c)";
  if (line.startsWith("▶")) return "var(--warning, #f39c12)";
  return "var(--text-primary)";
}

export function CompanyHeartbeatPanel({ workspacePath: _wp }: CompanyHeartbeatPanelProps) {
  const [history, setHistory] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [agentFilter, setAgentFilter] = useState("");
  const [triggerAgent, setTriggerAgent] = useState("");
  const [triggerResult, setTriggerResult] = useState<string | null>(null);
  const [limit, setLimit] = useState("20");

  const load = async () => {
    setLoading(true);
    try {
      const agentArg = agentFilter.trim();
      const limitArg = parseInt(limit) || 20;
      const args = agentArg
        ? `heartbeat history ${agentArg} ${limitArg}`
        : `heartbeat history "" ${limitArg}`;
      const out = await invoke<string>("company_cmd", { args });
      setHistory(out);
    } catch (e) {
      setHistory(`Error: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  const trigger = async () => {
    const agent = triggerAgent.trim();
    if (!agent) return;
    try {
      const out = await invoke<string>("company_heartbeat_trigger", { agentId: agent });
      setTriggerResult(out);
      load();
    } catch (e) {
      setTriggerResult(`Error: ${e}`);
    }
  };

  const lines = history.split("\n").filter(Boolean);

  return (
    <div style={{ padding: 16, fontSize: 13, height: "100%", overflowY: "auto" }}>
      {/* Header */}
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
        <span style={{ fontWeight: 600, fontSize: 14 }}>Heartbeats</span>
        <button onClick={load} style={btnStyle}>Refresh</button>
      </div>

      {/* Manual trigger */}
      <div style={{ background: "var(--panel-bg, rgba(0,0,0,0.2))", border: "1px solid var(--border-color)", borderRadius: 6, padding: 12, marginBottom: 14 }}>
        <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 8, fontWeight: 600 }}>MANUAL TRIGGER</div>
        <div style={{ display: "flex", gap: 8 }}>
          <input
            value={triggerAgent}
            onChange={(e) => setTriggerAgent(e.target.value)}
            placeholder="Agent ID"
            style={{ ...inputStyle, flex: 1 }}
          />
          <button
            onClick={trigger}
            disabled={!triggerAgent.trim()}
            style={{ ...btnStyle, padding: "4px 14px", border: "1px solid var(--warning, #f39c12)", color: "var(--warning, #f39c12)", opacity: triggerAgent.trim() ? 1 : 0.5, display: "inline-flex", alignItems: "center" }}
          >
            <Heart size={13} strokeWidth={1.5} style={{ marginRight: 4 }} /> Trigger
          </button>
        </div>
        {triggerResult && (
          <div style={{ marginTop: 8, fontSize: 12, padding: "6px 8px", background: "rgba(0,0,0,0.15)", borderRadius: 4, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span>{triggerResult}</span>
            <button onClick={() => setTriggerResult(null)} style={{ cursor: "pointer", background: "none", border: "none", color: "var(--text-secondary)", display: "inline-flex" }}><X size={12} /></button>
          </div>
        )}
      </div>

      {/* Filter bar */}
      <div style={{ display: "flex", gap: 8, marginBottom: 12, alignItems: "center" }}>
        <input
          value={agentFilter}
          onChange={(e) => setAgentFilter(e.target.value)}
          placeholder="Filter by Agent ID (blank = all)"
          style={{ ...inputStyle, flex: 1 }}
          onKeyDown={(e) => e.key === "Enter" && load()}
        />
        <input
          value={limit}
          onChange={(e) => setLimit(e.target.value)}
          type="number"
          placeholder="Limit"
          style={{ ...inputStyle, width: 64 }}
        />
        <button onClick={load} style={{ ...btnStyle, padding: "4px 12px" }}>Filter</button>
      </div>

      {/* Legend */}
      <div style={{ display: "flex", gap: 14, fontSize: 11, color: "var(--text-secondary)", marginBottom: 10 }}>
        <span style={{ color: "var(--warning, #f39c12)" }}>▶ running</span>
        <span style={{ color: "var(--success, #27ae60)", display: "inline-flex", alignItems: "center", gap: 3 }}><Check size={11} strokeWidth={2} /> completed</span>
        <span style={{ color: "var(--danger, #e74c3c)", display: "inline-flex", alignItems: "center", gap: 3 }}><X size={11} strokeWidth={2} /> failed</span>
        <span style={{ marginLeft: "auto" }}>
          {lines.length > 0 && !loading ? `${lines.length} run${lines.length !== 1 ? "s" : ""}` : ""}
        </span>
      </div>

      {/* History list */}
      <div style={{ background: "var(--panel-bg, rgba(0,0,0,0.2))", border: "1px solid var(--border-color)", borderRadius: 6, padding: 12, minHeight: 160 }}>
        {loading ? (
          <span style={{ color: "var(--text-secondary)" }}>Loading…</span>
        ) : lines.length === 0 ? (
          <span style={{ color: "var(--text-secondary)", fontSize: 12 }}>No heartbeat runs yet. Trigger one above.</span>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            {lines.map((line, i) => (
              <div
                key={i}
                style={{
                  fontSize: 12, fontFamily: "monospace", padding: "4px 6px",
                  borderRadius: 3, background: "rgba(0,0,0,0.15)",
                  color: statusColor(line), lineHeight: 1.5,
                }}
              >
                {line}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
