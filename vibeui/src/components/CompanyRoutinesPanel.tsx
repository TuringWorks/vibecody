/**
 * CompanyRoutinesPanel — Scheduled recurring agent tasks.
 *
 * Shows routines with next-run countdown and toggle switches.
 * Supports creating new routines and manual heartbeat triggers.
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface CompanyRoutinesPanelProps {
  workspacePath?: string | null;
}

export function CompanyRoutinesPanel({ workspacePath: _wp }: CompanyRoutinesPanelProps) {
  const [routineOutput, setRoutineOutput] = useState<string>("");
  const [heartbeatOutput, setHeartbeatOutput] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [agentId, setAgentId] = useState("");
  const [routineName, setRoutineName] = useState("");
  const [prompt, setPrompt] = useState("");
  const [intervalMin, setIntervalMin] = useState("60");
  const [cmdResult, setCmdResult] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    try {
      const out = await invoke<string>("company_cmd", { args: "routine list" });
      setRoutineOutput(out);
    } catch (e) {
      setRoutineOutput(`Error: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  const createRoutine = async () => {
    if (!agentId || !routineName || !prompt) return;
    const secs = parseInt(intervalMin) * 60;
    try {
      const out = await invoke<string>("company_cmd", {
        args: `routine create ${agentId} "${routineName}" --every ${secs} --prompt "${prompt}"`,
      });
      setCmdResult(out);
      setRoutineName("");
      setPrompt("");
      load();
    } catch (e) {
      setCmdResult(`Error: ${e}`);
    }
  };

  const triggerHeartbeat = async () => {
    if (!agentId) return;
    try {
      const out = await invoke<string>("company_cmd", { args: `heartbeat ${agentId}` });
      setHeartbeatOutput(out);
    } catch (e) {
      setHeartbeatOutput(`Error: ${e}`);
    }
  };

  return (
    <div style={{ padding: 16, fontSize: 13, height: "100%", overflowY: "auto" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
        <span style={{ fontWeight: 600, fontSize: 14 }}>Routines & Heartbeats</span>
        <button onClick={load} style={{ fontSize: 11, padding: "2px 8px", cursor: "pointer" }}>
          Refresh
        </button>
      </div>

      {/* Create routine */}
      <div style={{ marginBottom: 16, border: "1px solid var(--border)", borderRadius: 6, padding: 12 }}>
        <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 8 }}>Create Routine</div>
        <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
          <div style={{ display: "flex", gap: 6 }}>
            <input value={agentId} onChange={(e) => setAgentId(e.target.value)} placeholder="Agent ID"
              style={{ flex: 1, fontSize: 12, padding: "4px 8px", background: "var(--input-bg, rgba(0,0,0,0.3))", border: "1px solid var(--border)", borderRadius: 4, color: "var(--text-primary)" }} />
            <input value={routineName} onChange={(e) => setRoutineName(e.target.value)} placeholder="Routine name"
              style={{ flex: 1, fontSize: 12, padding: "4px 8px", background: "var(--input-bg, rgba(0,0,0,0.3))", border: "1px solid var(--border)", borderRadius: 4, color: "var(--text-primary)" }} />
            <input value={intervalMin} onChange={(e) => setIntervalMin(e.target.value)} placeholder="Minutes"
              type="number" style={{ width: 80, fontSize: 12, padding: "4px 8px", background: "var(--input-bg, rgba(0,0,0,0.3))", border: "1px solid var(--border)", borderRadius: 4, color: "var(--text-primary)" }} />
          </div>
          <div style={{ display: "flex", gap: 6 }}>
            <input value={prompt} onChange={(e) => setPrompt(e.target.value)} placeholder="Agent prompt/task"
              style={{ flex: 1, fontSize: 12, padding: "4px 8px", background: "var(--input-bg, rgba(0,0,0,0.3))", border: "1px solid var(--border)", borderRadius: 4, color: "var(--text-primary)" }} />
            <button onClick={createRoutine} style={{ fontSize: 11, padding: "4px 12px", cursor: "pointer" }}>
              Create
            </button>
          </div>
        </div>
      </div>

      {/* Manual heartbeat */}
      <div style={{ marginBottom: 16 }}>
        <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 6 }}>Manual Heartbeat</div>
        <div style={{ display: "flex", gap: 8 }}>
          <input value={agentId} onChange={(e) => setAgentId(e.target.value)} placeholder="Agent ID"
            style={{ flex: 1, fontSize: 12, padding: "4px 8px", background: "var(--input-bg, rgba(0,0,0,0.3))", border: "1px solid var(--border)", borderRadius: 4, color: "var(--text-primary)" }} />
          <button onClick={triggerHeartbeat} style={{ fontSize: 11, padding: "4px 12px", cursor: "pointer" }}>
            ♥ Trigger
          </button>
        </div>
        {heartbeatOutput && (
          <div style={{ marginTop: 8, fontSize: 12, padding: 8, background: "var(--panel-bg, rgba(0,0,0,0.2))", borderRadius: 4, border: "1px solid var(--border)" }}>
            {heartbeatOutput}
          </div>
        )}
      </div>

      {cmdResult && (
        <div style={{ background: "var(--panel-bg, rgba(0,0,0,0.2))", border: "1px solid var(--border)", borderRadius: 4, padding: 8, marginBottom: 12, fontSize: 12 }}>
          {cmdResult}
        </div>
      )}

      <div style={{ background: "var(--panel-bg, rgba(0,0,0,0.2))", border: "1px solid var(--border)", borderRadius: 6, padding: 12 }}>
        <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 6 }}>Active Routines</div>
        {loading ? (
          <span style={{ color: "var(--text-secondary)" }}>Loading…</span>
        ) : (
          <pre style={{ margin: 0, fontSize: 12, whiteSpace: "pre-wrap" }}>
            {routineOutput || "No routines. Create one above."}
          </pre>
        )}
      </div>
    </div>
  );
}
