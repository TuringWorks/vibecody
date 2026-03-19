/**
 * AgentTeamPanel — Agent Teams & Peer Communication.
 *
 * Launch a team of agents that collaborate on a shared goal.
 * Shows task decomposition, agent cards, and inter-agent message feed.
 */
import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface TeamTask {
  id: string;
  agent_id: string;
  description: string;
  status: string;
  result: string | null;
}

interface TeamMessage {
  from_agent_id: string;
  to_agent_id: string | null;
  msg_type: string;
  content: string;
  timestamp: number;
}

interface TeamInfo {
  id: string;
  lead_agent_id: string;
  member_ids: string[];
  goal: string;
  status: string;
  tasks: TeamTask[];
  message_count: number;
  messages: TeamMessage[];
}

type SubTab = "overview" | "messages" | "tasks";

const statusColor: Record<string, string> = {
  Pending: "var(--text-muted)",
  InProgress: "var(--accent-color)",
  Completed: "var(--success-color)",
  Failed: "var(--error-color)",
};

const msgTypeColor: Record<string, string> = {
  Finding: "var(--success-color)",
  Challenge: "var(--warning-color)",
  Request: "var(--accent-color)",
  Status: "var(--text-muted)",
  TaskAssignment: "var(--text-accent)",
  Ack: "var(--text-muted)",
};

export function AgentTeamPanel() {
  const [tab, setTab] = useState<SubTab>("overview");
  const [goal, setGoal] = useState("");
  const [memberCount, setMemberCount] = useState(3);
  const [team, setTeam] = useState<TeamInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const teamIdRef = useRef<string | null>(null);

  const refreshTeam = useCallback(async () => {
    if (!teamIdRef.current) return;
    try {
      const info = await invoke<TeamInfo>("get_team_status", { teamId: teamIdRef.current });
      setTeam(info);
    } catch {
      // Team may have been dismissed
    }
  }, []);

  // Listen for team:updated events from the backend
  useEffect(() => {
    const unlisten = listen("team:updated", () => { refreshTeam(); });
    return () => { unlisten.then((f) => f()); };
  }, [refreshTeam]);

  const handleCreate = async () => {
    if (!goal.trim()) { setError("Goal is required"); return; }
    setLoading(true);
    setError(null);
    try {
      const info = await invoke<TeamInfo>("start_agent_team", {
        goal: goal.trim(),
        memberCount,
      });
      teamIdRef.current = info.id;
      setTeam(info);
    } catch (e) {
      setError(String(e));
    }
    setLoading(false);
  };

  const handleDismiss = async () => {
    await invoke("dismiss_team").catch(() => {});
    teamIdRef.current = null;
    setTeam(null);
    setGoal("");
    setTab("overview");
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      {/* Header */}
      <div style={{
        padding: "8px 12px", borderBottom: "1px solid var(--border-color)",
        display: "flex", alignItems: "center", gap: 8,
      }}>
        <span style={{ fontSize: 14, fontWeight: 700 }}>Agent Teams</span>
        <div style={{ flex: 1 }} />
        {team && (
          <>
            <span style={{
              fontSize: 10, padding: "2px 8px", borderRadius: 10, fontWeight: 600,
              background: team.status === "working" ? "rgba(137,180,250,0.15)" : team.status === "complete" ? "rgba(52,211,153,0.15)" : "rgba(108,112,134,0.15)",
              color: team.status === "working" ? "var(--info-color)" : team.status === "complete" ? "var(--success-color)" : "var(--text-secondary)",
            }}>
              {team.status}
            </span>
            <button onClick={handleDismiss} style={{ ...btnStyle, fontSize: 10, padding: "2px 8px" }}>
              New Team
            </button>
          </>
        )}
      </div>

      {!team ? (
        /* Team creation form */
        <div style={{ padding: "16px 12px", display: "flex", flexDirection: "column", gap: 10 }}>
          <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>
            Create a team of AI agents that collaborate on a shared goal.
            The lead agent decomposes the task and coordinates members.
          </div>
          <div>
            <div style={labelStyle}>Goal</div>
            <textarea
              value={goal}
              onChange={(e) => setGoal(e.target.value)}
              rows={3}
              placeholder="Describe the goal for the team..."
              style={{ ...inputStyle, resize: "vertical", fontFamily: "inherit", width: "100%", boxSizing: "border-box" }}
            />
          </div>
          <div>
            <div style={labelStyle}>Team Size ({memberCount} agents)</div>
            <input
              type="range" min={2} max={8} value={memberCount}
              onChange={(e) => setMemberCount(parseInt(e.target.value))}
              style={{ width: "100%" }}
            />
          </div>
          <button onClick={handleCreate} disabled={loading || !goal.trim()} style={{
            ...btnStyle, background: "var(--accent-primary)", color: "var(--text-primary)", fontWeight: 700,
            opacity: loading || !goal.trim() ? 0.5 : 1,
          }}>
            {loading ? "Creating Team..." : "Create Team"}
          </button>
          {error && (
            <div style={{ fontSize: 11, color: "var(--text-danger)", padding: "4px 8px", background: "rgba(243,139,168,0.05)", borderRadius: 4 }}>
              {error}
            </div>
          )}
        </div>
      ) : (
        /* Team view */
        <>
          {/* Sub-tabs */}
          <div style={{ display: "flex", gap: 4, padding: "6px 12px", borderBottom: "1px solid var(--border-color)" }}>
            {(["overview", "tasks", "messages"] as const).map((t) => (
              <button
                key={t}
                onClick={() => setTab(t)}
                style={{
                  padding: "3px 10px", fontSize: 10, fontWeight: 600, borderRadius: 4, cursor: "pointer",
                  border: tab === t ? "1px solid var(--accent-color)" : "1px solid var(--border-color)",
                  background: tab === t ? "rgba(99,102,241,0.15)" : "transparent",
                  color: "var(--text-primary)",
                }}
              >
                {t === "overview" ? "Overview" : t === "tasks" ? `Tasks (${team.tasks.length})` : `Messages (${team.message_count})`}
              </button>
            ))}
          </div>

          <div style={{ flex: 1, overflowY: "auto", padding: "8px 12px" }}>
            {/* Overview tab */}
            {tab === "overview" && (
              <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
                <div style={{ fontSize: 12, fontWeight: 600 }}>Goal</div>
                <div style={{ fontSize: 11, padding: "6px 8px", background: "var(--bg-primary)", borderRadius: 4 }}>
                  {team.goal}
                </div>
                <div style={{ fontSize: 12, fontWeight: 600 }}>Members ({team.member_ids.length})</div>
                <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
                  {team.member_ids.map((id) => (
                    <div key={id} style={{
                      padding: "4px 8px", fontSize: 10, borderRadius: 4,
                      background: id === team.lead_agent_id ? "rgba(99,102,241,0.15)" : "var(--bg-primary)",
                      border: id === team.lead_agent_id ? "1px solid var(--accent-color)" : "1px solid var(--border-color)",
                    }}>
                      {id === team.lead_agent_id ? "Lead: " : ""}{id.split("-").pop()}
                    </div>
                  ))}
                </div>
                <div style={{ fontSize: 12, fontWeight: 600 }}>Progress</div>
                {team.tasks.length > 0 ? (
                  <div style={{ display: "flex", gap: 2, height: 8, borderRadius: 4, overflow: "hidden" }}>
                    {team.tasks.map((t) => (
                      <div key={t.id} style={{
                        flex: 1, background: statusColor[t.status] || "var(--text-muted)",
                        opacity: t.status === "Pending" ? 0.3 : 1,
                      }} title={`${t.description} (${t.status})`} />
                    ))}
                  </div>
                ) : (
                  <div style={{ fontSize: 11, opacity: 0.5 }}>No tasks decomposed yet</div>
                )}
              </div>
            )}

            {/* Tasks tab */}
            {tab === "tasks" && (
              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                {team.tasks.map((t) => (
                  <div key={t.id} style={{
                    padding: "6px 8px", borderRadius: 4,
                    border: "1px solid var(--border-color)",
                    background: "var(--bg-primary)",
                  }}>
                    <div style={{ display: "flex", gap: 6, alignItems: "center", marginBottom: 4 }}>
                      <span style={{
                        fontSize: 9, padding: "1px 6px", borderRadius: 3, fontWeight: 700,
                        background: statusColor[t.status] || "var(--text-muted)", color: "var(--bg-tertiary)",
                      }}>
                        {t.status}
                      </span>
                      <span style={{ fontSize: 10, opacity: 0.5 }}>{t.agent_id.split("-").pop()}</span>
                    </div>
                    <div style={{ fontSize: 11 }}>{t.description}</div>
                    {t.result && (
                      <div style={{ fontSize: 10, opacity: 0.7, marginTop: 4, fontStyle: "italic" }}>
                        {t.result}
                      </div>
                    )}
                  </div>
                ))}
                {team.tasks.length === 0 && (
                  <div style={{ padding: 16, textAlign: "center", opacity: 0.5, fontSize: 11 }}>
                    No tasks assigned yet. The lead agent will decompose the goal.
                  </div>
                )}
              </div>
            )}

            {/* Messages tab */}
            {tab === "messages" && (
              <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                {team.messages.map((m, i) => (
                  <div key={i} style={{
                    padding: "4px 8px", borderRadius: 4,
                    borderLeft: `3px solid ${msgTypeColor[m.msg_type] || "var(--text-muted)"}`,
                    background: "var(--bg-primary)",
                  }}>
                    <div style={{ display: "flex", gap: 6, alignItems: "center", marginBottom: 2 }}>
                      <span style={{ fontSize: 9, fontWeight: 700, color: msgTypeColor[m.msg_type] || "var(--text-muted)" }}>
                        {m.msg_type}
                      </span>
                      <span style={{ fontSize: 9, opacity: 0.5 }}>
                        from {m.from_agent_id.split("-").pop()}
                        {m.to_agent_id ? ` → ${m.to_agent_id.split("-").pop()}` : ""}
                      </span>
                      <div style={{ flex: 1 }} />
                      <span style={{ fontSize: 9, opacity: 0.3, fontFamily: "var(--font-mono)" }}>
                        {new Date(m.timestamp).toLocaleTimeString()}
                      </span>
                    </div>
                    <div style={{ fontSize: 11 }}>{m.content}</div>
                  </div>
                ))}
                {team.messages.length === 0 && (
                  <div style={{ padding: 16, textAlign: "center", opacity: 0.5, fontSize: 11 }}>
                    No messages yet. Agents will communicate as they work.
                  </div>
                )}
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
}

const btnStyle: React.CSSProperties = {
  padding: "6px 12px", fontSize: 12, fontWeight: 600,
  border: "1px solid var(--border-color)", borderRadius: 4,
  background: "var(--bg-secondary)", color: "var(--text-primary)",
  cursor: "pointer",
};

const inputStyle: React.CSSProperties = {
  padding: "5px 8px", fontSize: 11, borderRadius: 4,
  border: "1px solid var(--border-color)",
  background: "var(--bg-primary)", color: "var(--text-primary)",
  outline: "none",
};

const labelStyle: React.CSSProperties = {
  fontSize: 10, fontWeight: 600, marginBottom: 3,
  color: "var(--text-secondary)",
  textTransform: "uppercase" as const, letterSpacing: "0.06em",
};
