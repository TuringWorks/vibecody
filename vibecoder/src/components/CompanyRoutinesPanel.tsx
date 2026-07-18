/**
 * CompanyRoutinesPanel — Scheduled recurring agent tasks.
 *
 * Shows routines with next-run countdown and toggle switches.
 * Supports creating new routines with delivery mode and skill selection.
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Heart } from "lucide-react";

interface CompanyRoutinesPanelProps {
  workspacePath?: string | null;
}

interface Routine {
  id: string;
  agent_id: string;
  name: string;
  prompt: string;
  interval_secs: number;
  delivery_mode: 'none' | 'announce' | 'interrupt';
  skill_name: string | null;
  enabled: boolean;
  next_run_at: number | null;
}

interface SkillSummary {
  name: string;
  description: string;
  filename: string;
}


function deliveryBadgeStyle(mode: Routine['delivery_mode']): React.CSSProperties {
  const color = mode === 'none' ? 'var(--text-secondary)' : mode === 'announce' ? 'var(--accent-blue)' : 'var(--accent-gold)';
  const bg = mode === 'none' ? 'rgba(128,128,128,0.15)' : mode === 'announce' ? 'rgba(74,158,255,0.15)' : 'rgba(255,193,7,0.15)';
  return {
    display: 'inline-block', padding: '1px 7px', borderRadius: "var(--radius-md)",
    fontSize: "var(--font-size-xs)", fontWeight: 600, color, background: bg, border: `1px solid ${color}`,
  };
}

const DELIVERY_DESCRIPTIONS: Record<string, string> = {
  none: 'Silent',
  announce: 'Post update to channel',
  interrupt: 'Notify principal immediately',
};

export function CompanyRoutinesPanel({ workspacePath: _wp }: CompanyRoutinesPanelProps) {
  const [routines, setRoutines] = useState<Routine[]>([]);
  const [skills, setSkills] = useState<SkillSummary[]>([]);
  const [heartbeatOutput, setHeartbeatOutput] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [agentId, setAgentId] = useState("");
  const [routineName, setRoutineName] = useState("");
  const [prompt, setPrompt] = useState("");
  const [intervalMin, setIntervalMin] = useState("60");
  const [deliveryMode, setDeliveryMode] = useState<'none' | 'announce' | 'interrupt'>('none');
  const [skillName, setSkillName] = useState<string>("");
  const [cmdResult, setCmdResult] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    try {
      const out = await invoke<Routine[]>("company_routine_list_json");
      setRoutines(out);
    } catch (_e) {
      setRoutines([]);
    } finally {
      setLoading(false);
    }
  };

  const loadSkills = async () => {
    try {
      const out = await invoke<SkillSummary[]>("company_list_skills");
      setSkills(out);
    } catch (_e) {
      setSkills([]);
    }
  };

  useEffect(() => {
    load();
    loadSkills();
  }, []);

  const createRoutine = async () => {
    if (!agentId || !routineName || !prompt) return;
    const intervalSecs = parseInt(intervalMin) * 60;
    try {
      await invoke("company_routine_create_v2", {
        agentId,
        name: routineName,
        intervalSecs,
        prompt,
        deliveryMode,
        skillName: skillName || null,
      });
      setCmdResult("Routine created.");
      setRoutineName("");
      setPrompt("");
      setDeliveryMode('none');
      setSkillName("");
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

  const promptPlaceholder = skillName
    ? "Skill prompt will be used. Add extra context here (optional)."
    : "Agent prompt/task";

  return (
    <div className="panel-container">
      <div className="panel-header">
        <h3>Routines & Heartbeats</h3>
        <button onClick={load} className="panel-btn panel-btn-secondary" style={{ marginLeft: "auto" }}>
          Refresh
        </button>
      </div>

      <div className="panel-body">
        {/* Create routine */}
        <div className="panel-card" style={{ marginBottom: 16 }}>
          <div className="panel-label" style={{ marginBottom: 8, fontWeight: 600 }}>Create Routine</div>
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            <div style={{ display: "flex", gap: 6 }}>
              <input value={agentId} onChange={(e) => setAgentId(e.target.value)} placeholder="Agent ID"
                className="panel-input" style={{ flex: 1 }} />
              <input value={routineName} onChange={(e) => setRoutineName(e.target.value)} placeholder="Routine name"
                className="panel-input" style={{ flex: 1 }} />
              <input value={intervalMin} onChange={(e) => setIntervalMin(e.target.value)} placeholder="Min"
                type="number" className="panel-input" style={{ width: 70 }} />
            </div>
            {/* Delivery mode */}
            <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
              <label style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", whiteSpace: "nowrap" }}>Delivery:</label>
              <select
                value={deliveryMode}
                onChange={(e) => setDeliveryMode(e.target.value as typeof deliveryMode)}
                className="panel-select"
                style={{ flex: 1 }}
              >
                <option value="none">None — {DELIVERY_DESCRIPTIONS.none}</option>
                <option value="announce">Announce — {DELIVERY_DESCRIPTIONS.announce}</option>
                <option value="interrupt">Interrupt — {DELIVERY_DESCRIPTIONS.interrupt}</option>
              </select>
            </div>
            {/* Skill select */}
            <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
              <label style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", whiteSpace: "nowrap" }}>Skill:</label>
              <select
                value={skillName}
                onChange={(e) => setSkillName(e.target.value)}
                className="panel-select"
                style={{ flex: 1 }}
              >
                <option value="">— None (use prompt) —</option>
                {skills.map((s) => (
                  <option key={s.filename} value={s.name} title={s.description}>{s.name}</option>
                ))}
              </select>
            </div>
            {/* Prompt / context */}
            <div style={{ display: "flex", gap: 6 }}>
              <textarea
                value={prompt}
                onChange={(e) => setPrompt(e.target.value)}
                placeholder={promptPlaceholder}
                rows={2}
                className="panel-input panel-textarea"
                style={{ flex: 1 }}
              />
              <button onClick={createRoutine} className="panel-btn panel-btn-primary" style={{ alignSelf: "flex-start" }}>
                Create
              </button>
            </div>
          </div>
        </div>

        {/* Manual heartbeat */}
        <div style={{ marginBottom: 16 }}>
          <div className="panel-label" style={{ marginBottom: 6 }}>Manual Heartbeat</div>
          <div style={{ display: "flex", gap: 8 }}>
            <input value={agentId} onChange={(e) => setAgentId(e.target.value)} placeholder="Agent ID"
              className="panel-input" style={{ flex: 1 }} />
            <button onClick={triggerHeartbeat} className="panel-btn panel-btn-secondary" style={{ display: "inline-flex", alignItems: "center" }}>
              <Heart size={13} strokeWidth={1.5} style={{ marginRight: 4 }} /> Trigger
            </button>
          </div>
          {heartbeatOutput && (
            <div className="panel-card" style={{ marginTop: 8, fontSize: "var(--font-size-base)" }}>
              {heartbeatOutput}
            </div>
          )}
        </div>

        {cmdResult && (
          <div className="panel-card" style={{ marginBottom: 12, fontSize: "var(--font-size-base)" }}>
            {cmdResult}
          </div>
        )}

        {/* Routine cards */}
        <div className="panel-card" style={{ padding: 0, overflow: "hidden" }}>
          <div className="panel-label" style={{ padding: "8px 12px", borderBottom: "1px solid var(--border-color)", fontWeight: 600 }}>Active Routines</div>
          {loading ? (
            <div className="panel-loading" style={{ padding: 16 }}>Loading…</div>
          ) : routines.length === 0 ? (
            <div style={{ padding: 16, fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>No routines. Create one above.</div>
          ) : (
            <div style={{ display: "flex", flexDirection: "column" }}>
              {routines.map((r) => (
                <div key={r.id} style={{ padding: "12px 12px", borderBottom: "1px solid var(--border-color)" }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
                    <strong style={{ fontSize: "var(--font-size-md)" }}>{r.name}</strong>
                    <span style={deliveryBadgeStyle(r.delivery_mode)}>{r.delivery_mode}</span>
                    {r.skill_name && (
                      <span style={{ fontSize: "var(--font-size-xs)", padding: '1px 8px', borderRadius: "var(--radius-md)", background: 'rgba(128,128,128,0.15)', color: 'var(--text-secondary)', border: '1px solid var(--border-color)' }}>
                        skill: {r.skill_name}
                      </span>
                    )}
                    <span style={{ marginLeft: "auto", fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
                      {r.enabled ? 'enabled' : 'disabled'}
                    </span>
                  </div>
                  <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
                    Agent: {r.agent_id} · Every {Math.round(r.interval_secs / 60)}min
                    {r.next_run_at ? ` · Next: ${new Date(r.next_run_at).toLocaleTimeString()}` : ''}
                  </div>
                  {r.prompt && (
                    <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: 4, fontStyle: "italic" }}>
                      "{r.prompt}"
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
