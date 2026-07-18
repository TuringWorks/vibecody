/**
 * CompanyHeartbeatPanel — Agent heartbeat run history and manual triggers.
 *
 * Shows company-wide or per-agent heartbeat runs with status badges,
 * trigger type, duration, and summary. Supports manual trigger.
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Heart, X, ChevronDown, ChevronRight } from "lucide-react";

interface CompanyHeartbeatPanelProps {
  workspacePath?: string | null;
}

interface HeartbeatRun {
  id: string;
  company_id: string;
  agent_id: string;
  trigger: string;
  status: 'running' | 'completed' | 'failed';
  session_id: string | null;
  started_at: number;
  finished_at: number | null;
  summary: string | null;
}


function statusBadgeStyle(status: HeartbeatRun['status']): React.CSSProperties {
  const color =
    status === 'running' ? 'var(--accent-blue)' :
    status === 'completed' ? 'var(--accent-green)' :
    'var(--accent-rose)';
  return {
    display: 'inline-block', padding: '1px 7px', borderRadius: "var(--radius-md)", fontSize: "var(--font-size-xs)",
    fontWeight: 700, background: color, color: '#fff', textTransform: 'uppercase',
  };
}

function formatDuration(run: HeartbeatRun): string {
  if (run.finished_at == null) return '…';
  const ms = run.finished_at - run.started_at;
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}

function formatTs(ts: number): string {
  return new Date(ts).toLocaleString();
}

export function CompanyHeartbeatPanel({ workspacePath: _wp }: CompanyHeartbeatPanelProps) {
  const [runs, setRuns] = useState<HeartbeatRun[]>([]);
  const [loading, setLoading] = useState(false);
  const [agentFilter, setAgentFilter] = useState("");
  const [triggerAgent, setTriggerAgent] = useState("");
  const [triggerResult, setTriggerResult] = useState<string | null>(null);
  const [expandedId, setExpandedId] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    try {
      const filter = agentFilter.trim() || null;
      const result = await invoke<HeartbeatRun[]>("company_heartbeat_history_json", {
        agentId: filter,
        limit: 50,
      });
      setRuns(result);
    } catch (_e) {
      setRuns([]);
    } finally {
      setLoading(false);
    }
  };

  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(() => { load(); }, []);

  const trigger = async () => {
    const agent = triggerAgent.trim();
    if (!agent) return;
    try {
      const out = await invoke<string>("company_heartbeat_trigger", { agentId: agent });
      setTriggerResult(out);
      load();
    } catch (_e) {
      setTriggerResult(`Error: ${_e}`);
    }
  };

  return (
    <div className="panel-container">
      {/* Header */}
      <div className="panel-header">
        <h3>Heartbeats</h3>
        <button onClick={load} className="panel-btn panel-btn-secondary" style={{ marginLeft: "auto" }}>Refresh</button>
      </div>
      <div className="panel-body">

      {/* Manual trigger */}
      <div className="panel-card" style={{ marginBottom: 14 }}>
        <div className="panel-label" style={{ marginBottom: 8, fontWeight: 600 }}>MANUAL TRIGGER</div>
        <div style={{ display: "flex", gap: 8 }}>
          <input
            value={triggerAgent}
            onChange={(e) => setTriggerAgent(e.target.value)}
            placeholder="Agent ID"
            className="panel-input"
            style={{ flex: 1 }}
          />
          <button
            onClick={trigger}
            disabled={!triggerAgent.trim()}
            className="panel-btn panel-btn-secondary"
            style={{ border: "1px solid var(--warning, #f39c12)", color: "var(--warning, #f39c12)", opacity: triggerAgent.trim() ? 1 : 0.5, display: "inline-flex", alignItems: "center" }}
          >
            <Heart size={13} strokeWidth={1.5} style={{ marginRight: 4 }} /> Trigger
          </button>
        </div>
        {triggerResult && (
          <div style={{ marginTop: 8, fontSize: "var(--font-size-base)", padding: "8px 8px", background: "rgba(0,0,0,0.15)", borderRadius: "var(--radius-xs-plus)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span>{triggerResult}</span>
            <button onClick={() => setTriggerResult(null)} style={{ cursor: "pointer", background: "none", border: "none", color: "var(--text-secondary)", display: "inline-flex" }} aria-label="Dismiss message"><X size={12} /></button>
          </div>
        )}
      </div>

      {/* Filter bar */}
      <div style={{ display: "flex", gap: 8, marginBottom: 12, alignItems: "center" }}>
        <input
          value={agentFilter}
          onChange={(e) => setAgentFilter(e.target.value)}
          placeholder="Filter by Agent ID (blank = all)"
          className="panel-input"
          style={{ flex: 1 }}
          onKeyDown={(e) => e.key === "Enter" && load()}
        />
        <button onClick={load} className="panel-btn panel-btn-secondary" style={{ padding: "4px 12px" }}>Filter</button>
      </div>

      {/* Legend */}
      <div style={{ display: "flex", gap: 14, fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 10 }}>
        <span style={{ color: "var(--accent-blue)" }}>● running</span>
        <span style={{ color: "var(--accent-green)" }}>● completed</span>
        <span style={{ color: "var(--accent-rose)" }}>● failed</span>
        <span style={{ marginLeft: "auto" }}>
          {runs.length > 0 && !loading ? `${runs.length} run${runs.length !== 1 ? "s" : ""}` : ""}
        </span>
      </div>

      {/* History list */}
      <div className="panel-card" style={{ minHeight: 160, padding: 0, overflow: "hidden" }}>
        {loading ? (
          <span className="panel-loading" style={{ padding: 16 }}>Loading…</span>
        ) : runs.length === 0 ? (
          <span className="panel-empty" style={{ padding: 16, display: "block" }}>No heartbeat runs yet. Trigger one above.</span>
        ) : (
          <div style={{ display: "flex", flexDirection: "column" }}>
            {runs.map((run) => {
              const expanded = expandedId === run.id;
              return (
                <div key={run.id} style={{ borderBottom: "1px solid var(--border-color)" }}>
                  {/* Row */}
                  <div role="button" tabIndex={0}
                    onClick={() => setExpandedId(expanded ? null : run.id)}
                    style={{
                      display: "flex", alignItems: "center", gap: 10, padding: "8px 12px",
                      cursor: "pointer", background: expanded ? "var(--bg-secondary)" : "transparent",
                    }}
                  >
                    <span style={{ color: "var(--text-secondary)", display: "inline-flex" }}>
                      {expanded ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
                    </span>
                    <span style={statusBadgeStyle(run.status)}>{run.status}</span>
                    <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", minWidth: 70 }}>{run.trigger}</span>
                    <span style={{ fontSize: "var(--font-size-base)", fontFamily: "var(--font-mono)", flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{run.agent_id}</span>
                    <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", minWidth: 48, textAlign: "right" }}>{formatDuration(run)}</span>
                    <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", minWidth: 140, textAlign: "right" }}>{formatTs(run.started_at)}</span>
                  </div>
                  {/* Inspector */}
                  {expanded && (
                    <div style={{
                      padding: "12px 16px 16px", background: "var(--bg-tertiary)",
                      fontSize: "var(--font-size-base)", borderTop: "1px solid var(--border-color)",
                    }}>
                      {run.summary && (
                        <div style={{ marginBottom: 10, lineHeight: 1.6 }}>
                          <span style={{ fontSize: "var(--font-size-sm)", fontWeight: 600, color: "var(--text-secondary)", display: "block", marginBottom: 4 }}>SUMMARY</span>
                          <div style={{ whiteSpace: "pre-wrap" }}>{run.summary}</div>
                        </div>
                      )}
                      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "4px 16px", fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
                        <div><strong>Run ID:</strong> <span style={{ fontFamily: "var(--font-mono)" }}>{run.id}</span></div>
                        <div><strong>Company:</strong> <span style={{ fontFamily: "var(--font-mono)" }}>{run.company_id}</span></div>
                        <div><strong>Agent:</strong> <span style={{ fontFamily: "var(--font-mono)" }}>{run.agent_id}</span></div>
                        <div><strong>Trigger:</strong> {run.trigger}</div>
                        <div><strong>Session ID:</strong> <span style={{ fontFamily: "var(--font-mono)" }}>{run.session_id ?? '—'}</span></div>
                        <div><strong>Status:</strong> {run.status}</div>
                        <div><strong>Started:</strong> {formatTs(run.started_at)}</div>
                        <div><strong>Finished:</strong> {run.finished_at ? formatTs(run.finished_at) : '—'}</div>
                        <div><strong>Duration:</strong> {formatDuration(run)}</div>
                      </div>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>
      </div>
    </div>
  );
}
