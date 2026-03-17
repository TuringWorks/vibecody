/**
 * AgentTeamsPanel — Agent Teams management with real Tauri backend.
 *
 * Tabs: Team (create/manage teams), Messages (inter-agent communication),
 * Tasks (task decomposition and progress), History (past team runs).
 * All data flows through Tauri invoke() calls backed by vibe-ai agent_team.
 */
import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

type Tab = "team" | "messages" | "tasks" | "history";

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

interface TeamHistoryEntry {
  id: string;
  goal: string;
  status: string;
  member_count: number;
  task_count: number;
  completed_at: string;
}

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
  TaskAssignment: "var(--text-info)",
  Ack: "var(--text-muted)",
  Info: "var(--accent-color)",
  Update: "var(--success-color)",
  Alert: "var(--error-color)",
};

const statusBadge: Record<string, { bg: string; color: string }> = {
  working: { bg: "rgba(137,180,250,0.15)", color: "var(--info-color)" },
  complete: { bg: "rgba(52,211,153,0.15)", color: "var(--success-color)" },
  failed: { bg: "rgba(239,68,68,0.15)", color: "var(--error-color)" },
};

const AgentTeamsPanel: React.FC = () => {
  const [tab, setTab] = useState<Tab>("team");
  const [goal, setGoal] = useState("");
  const [memberCount, setMemberCount] = useState(3);
  const [team, setTeam] = useState<TeamInfo | null>(null);
  const [history, setHistory] = useState<TeamHistoryEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [userMsg, setUserMsg] = useState("");
  const teamIdRef = useRef<string | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const refreshTeam = useCallback(async () => {
    if (!teamIdRef.current) return;
    try {
      const info = await invoke<TeamInfo>("get_team_status", { teamId: teamIdRef.current });
      setTeam(info);
    } catch {
      // Team may have been dismissed
    }
  }, []);

  const loadHistory = useCallback(async () => {
    try {
      const h = await invoke<TeamHistoryEntry[]>("get_team_history");
      setHistory(h);
    } catch {
      // History command may not exist yet — fallback to empty
      setHistory([]);
    }
  }, []);

  useEffect(() => {
    loadHistory();
  }, [loadHistory]);

  useEffect(() => {
    const unlisten = listen("team:updated", () => { refreshTeam(); });
    return () => { unlisten.then((f) => f()); };
  }, [refreshTeam]);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [team?.messages]);

  const handleCreate = async () => {
    if (!goal.trim()) { setError("Please enter a goal for the team"); return; }
    setLoading(true);
    setError(null);
    try {
      const info = await invoke<TeamInfo>("start_agent_team", {
        goal: goal.trim(),
        memberCount,
      });
      teamIdRef.current = info.id;
      setTeam(info);
      setTab("tasks");
    } catch (e) {
      setError(String(e));
    }
    setLoading(false);
  };

  const handleDismiss = async () => {
    if (team) {
      // Save to history before dismissing
      setHistory(prev => [{
        id: team.id,
        goal: team.goal,
        status: team.status,
        member_count: team.member_ids.length,
        task_count: team.tasks.length,
        completed_at: new Date().toISOString(),
      }, ...prev]);
    }
    await invoke("dismiss_team").catch(() => {});
    teamIdRef.current = null;
    setTeam(null);
    setGoal("");
    setTab("team");
  };

  const handleSendMessage = async () => {
    if (!userMsg.trim() || !teamIdRef.current) return;
    try {
      await invoke("send_team_message", {
        teamId: teamIdRef.current,
        content: userMsg.trim(),
      });
      setUserMsg("");
      await refreshTeam();
    } catch (e) {
      setError(String(e));
    }
  };

  const completedTasks = team?.tasks.filter(t => t.status === "Completed").length ?? 0;
  const totalTasks = team?.tasks.length ?? 0;
  const progressPct = totalTasks > 0 ? Math.round((completedTasks / totalTasks) * 100) : 0;

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden", color: "var(--text-primary)" }}>
      {/* Header */}
      <div style={{
        padding: "8px 12px", borderBottom: "1px solid var(--border-color)",
        display: "flex", alignItems: "center", gap: 8,
      }}>
        <span style={{ fontSize: 14, fontWeight: 700 }}>Agent Teams</span>
        {team && (
          <>
            <span style={{
              fontSize: 10, padding: "2px 8px", borderRadius: 10, fontWeight: 600,
              background: (statusBadge[team.status] ?? statusBadge.working).bg,
              color: (statusBadge[team.status] ?? statusBadge.working).color,
            }}>
              {team.status}
            </span>
            <span style={{ fontSize: 10, color: "var(--text-muted)" }}>
              {completedTasks}/{totalTasks} tasks
            </span>
          </>
        )}
        <div style={{ flex: 1 }} />
        {team && (
          <button onClick={handleDismiss} style={btnSecondary}>
            New Team
          </button>
        )}
      </div>

      {/* Tab bar */}
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--border-color)" }}>
        {([
          ["team", "Team"],
          ["tasks", `Tasks${team ? ` (${totalTasks})` : ""}`],
          ["messages", `Messages${team ? ` (${team.message_count})` : ""}`],
          ["history", `History${history.length ? ` (${history.length})` : ""}`],
        ] as [Tab, string][]).map(([id, label]) => (
          <button
            key={id}
            onClick={() => setTab(id)}
            style={{
              padding: "6px 14px", cursor: "pointer", fontSize: 12,
              borderBottom: tab === id ? "2px solid var(--accent-color)" : "2px solid transparent",
              color: tab === id ? "var(--accent-color)" : "var(--text-secondary)",
              background: "none", border: "none", borderBottomStyle: "solid",
            }}
          >
            {label}
          </button>
        ))}
      </div>

      {/* Error banner */}
      {error && (
        <div style={{
          padding: "6px 12px", background: "var(--error-bg)", borderBottom: "1px solid var(--error-color)",
          display: "flex", alignItems: "center", gap: 8, fontSize: 11, color: "var(--text-danger)",
        }}>
          <span style={{ flex: 1 }}>{error}</span>
          <button onClick={() => setError(null)} style={{ ...btnSecondary, fontSize: 10, padding: "2px 6px" }}>Dismiss</button>
        </div>
      )}

      {/* Content area */}
      <div style={{ flex: 1, overflowY: "auto", padding: "10px 12px" }}>

        {/* TEAM TAB */}
        {tab === "team" && !team && (
          <div style={{ display: "flex", flexDirection: "column", gap: 12, maxWidth: 500 }}>
            <div style={{ fontSize: 12, color: "var(--text-secondary)", lineHeight: 1.5 }}>
              Create a team of AI agents that collaborate on a shared goal.
              The lead agent decomposes the task into sub-tasks and coordinates team members.
              Each agent works autonomously and communicates findings via the message bus.
            </div>
            <div>
              <div style={labelStyle}>Goal</div>
              <textarea
                value={goal}
                onChange={(e) => setGoal(e.target.value)}
                rows={4}
                placeholder="e.g., Refactor the authentication module to use JWT tokens with refresh rotation..."
                style={{ ...inputStyle, resize: "vertical", fontFamily: "inherit", width: "100%", boxSizing: "border-box" }}
              />
            </div>
            <div>
              <div style={labelStyle}>Team Size</div>
              <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                <input
                  type="range" min={2} max={8} value={memberCount}
                  onChange={(e) => setMemberCount(parseInt(e.target.value))}
                  style={{ flex: 1 }}
                />
                <span style={{ fontSize: 13, fontWeight: 700, fontFamily: "monospace", minWidth: 60 }}>
                  {memberCount} agents
                </span>
              </div>
              <div style={{ fontSize: 10, color: "var(--text-muted)", marginTop: 4 }}>
                1 lead + {memberCount - 1} worker{memberCount - 1 !== 1 ? "s" : ""}
              </div>
            </div>
            <button onClick={handleCreate} disabled={loading || !goal.trim()} style={{
              ...btnPrimary,
              opacity: loading || !goal.trim() ? 0.5 : 1,
              cursor: loading || !goal.trim() ? "not-allowed" : "pointer",
            }}>
              {loading ? "Creating Team & Decomposing Goal..." : "Create Team"}
            </button>
          </div>
        )}

        {tab === "team" && team && (
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            {/* Goal */}
            <div style={{ padding: "10px 12px", background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)" }}>
              <div style={{ fontSize: 10, fontWeight: 600, color: "var(--text-muted)", marginBottom: 4, textTransform: "uppercase", letterSpacing: "0.05em" }}>Goal</div>
              <div style={{ fontSize: 12 }}>{team.goal}</div>
            </div>

            {/* Progress bar */}
            <div>
              <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
                <span style={{ fontSize: 10, color: "var(--text-muted)" }}>Progress</span>
                <span style={{ fontSize: 10, fontWeight: 600, color: "var(--text-info)" }}>{progressPct}%</span>
              </div>
              <div style={{ height: 8, background: "var(--bg-primary)", borderRadius: 4, overflow: "hidden" }}>
                <div style={{
                  width: `${progressPct}%`, height: "100%", borderRadius: 4,
                  background: progressPct === 100 ? "var(--success-color)" : "var(--accent-color)",
                  transition: "width 0.5s ease",
                }} />
              </div>
            </div>

            {/* Members */}
            <div>
              <div style={{ fontSize: 10, fontWeight: 600, color: "var(--text-muted)", marginBottom: 6, textTransform: "uppercase", letterSpacing: "0.05em" }}>
                Members ({team.member_ids.length})
              </div>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
                {team.member_ids.map((id) => {
                  const isLead = id === team.lead_agent_id;
                  const agentTasks = team.tasks.filter(t => t.agent_id === id);
                  const completed = agentTasks.filter(t => t.status === "Completed").length;
                  const inProgress = agentTasks.filter(t => t.status === "InProgress").length;
                  return (
                    <div key={id} style={{
                      padding: "6px 10px", borderRadius: 6, minWidth: 120,
                      background: isLead ? "rgba(99,102,241,0.1)" : "var(--bg-secondary)",
                      border: `1px solid ${isLead ? "var(--accent-color)" : "var(--border-color)"}`,
                    }}>
                      <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 2 }}>
                        {isLead ? "Lead" : id.split("-").pop()}
                      </div>
                      <div style={{ fontSize: 10, color: "var(--text-muted)" }}>
                        {inProgress > 0 && <span style={{ color: "var(--accent-color)" }}>Working </span>}
                        {completed > 0 && <span style={{ color: "var(--success-color)" }}>{completed} done </span>}
                        {agentTasks.length === 0 && "Idle"}
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          </div>
        )}

        {/* TASKS TAB */}
        {tab === "tasks" && !team && (
          <div style={{ textAlign: "center", padding: 40, color: "var(--text-muted)", fontSize: 12 }}>
            Create a team first to see task decomposition.
          </div>
        )}

        {tab === "tasks" && team && (
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            {team.tasks.length === 0 && (
              <div style={{ textAlign: "center", padding: 30, color: "var(--text-muted)", fontSize: 12 }}>
                The lead agent is decomposing the goal into sub-tasks...
              </div>
            )}
            {team.tasks.map((t) => (
              <div key={t.id} style={{
                padding: "8px 10px", borderRadius: 6,
                border: "1px solid var(--border-color)",
                background: "var(--bg-secondary)",
                borderLeft: `3px solid ${statusColor[t.status] ?? "var(--text-muted)"}`,
              }}>
                <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 4 }}>
                  <span style={{
                    fontSize: 9, padding: "1px 6px", borderRadius: 3, fontWeight: 700,
                    background: `${statusColor[t.status] ?? "var(--text-muted)"}22`,
                    color: statusColor[t.status] ?? "var(--text-muted)",
                  }}>
                    {t.status}
                  </span>
                  <span style={{ fontSize: 10, color: "var(--text-muted)" }}>
                    {t.agent_id === team.lead_agent_id ? "Lead" : t.agent_id.split("-").pop()}
                  </span>
                </div>
                <div style={{ fontSize: 12, marginBottom: t.result ? 6 : 0 }}>{t.description}</div>
                {t.result && (
                  <div style={{
                    fontSize: 11, color: "var(--text-muted)", marginTop: 4,
                    padding: "6px 8px", background: "var(--bg-primary)", borderRadius: 4,
                    fontStyle: "italic", lineHeight: 1.4,
                  }}>
                    {t.result}
                  </div>
                )}
              </div>
            ))}
          </div>
        )}

        {/* MESSAGES TAB */}
        {tab === "messages" && !team && (
          <div style={{ textAlign: "center", padding: 40, color: "var(--text-muted)", fontSize: 12 }}>
            Create a team first to see inter-agent messages.
          </div>
        )}

        {tab === "messages" && team && (
          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            {team.messages.length === 0 && (
              <div style={{ textAlign: "center", padding: 30, color: "var(--text-muted)", fontSize: 12 }}>
                No messages yet. Agents will communicate as they work.
              </div>
            )}
            {team.messages.map((m, i) => (
              <div key={i} style={{
                padding: "6px 10px", borderRadius: 4,
                borderLeft: `3px solid ${msgTypeColor[m.msg_type] ?? "var(--text-muted)"}`,
                background: "var(--bg-secondary)",
              }}>
                <div style={{ display: "flex", gap: 6, alignItems: "center", marginBottom: 2 }}>
                  <span style={{ fontSize: 9, fontWeight: 700, color: msgTypeColor[m.msg_type] ?? "var(--text-muted)" }}>
                    {m.msg_type}
                  </span>
                  <span style={{ fontSize: 9, color: "var(--text-muted)" }}>
                    {m.from_agent_id === team.lead_agent_id ? "Lead" : m.from_agent_id.split("-").pop()}
                    {m.to_agent_id ? ` → ${m.to_agent_id === team.lead_agent_id ? "Lead" : m.to_agent_id.split("-").pop()}` : " → all"}
                  </span>
                  <div style={{ flex: 1 }} />
                  <span style={{ fontSize: 9, color: "var(--text-muted)", fontFamily: "monospace", opacity: 0.6 }}>
                    {new Date(m.timestamp).toLocaleTimeString()}
                  </span>
                </div>
                <div style={{ fontSize: 11, lineHeight: 1.4 }}>{m.content}</div>
              </div>
            ))}
            <div ref={messagesEndRef} />
          </div>
        )}

        {/* HISTORY TAB */}
        {tab === "history" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            {history.length === 0 && (
              <div style={{ textAlign: "center", padding: 40, color: "var(--text-muted)", fontSize: 12 }}>
                No past team runs yet. Completed teams will appear here.
              </div>
            )}
            {history.map((h) => (
              <div key={h.id} style={{
                padding: "8px 10px", borderRadius: 6,
                border: "1px solid var(--border-color)",
                background: "var(--bg-secondary)",
              }}>
                <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
                  <span style={{
                    fontSize: 9, padding: "1px 6px", borderRadius: 3, fontWeight: 700,
                    background: (statusBadge[h.status] ?? statusBadge.complete).bg,
                    color: (statusBadge[h.status] ?? statusBadge.complete).color,
                  }}>
                    {h.status}
                  </span>
                  <span style={{ fontSize: 10, color: "var(--text-muted)" }}>
                    {h.member_count} agents · {h.task_count} tasks
                  </span>
                  <div style={{ flex: 1 }} />
                  <span style={{ fontSize: 10, color: "var(--text-muted)", fontFamily: "monospace" }}>
                    {new Date(h.completed_at).toLocaleDateString()}
                  </span>
                </div>
                <div style={{ fontSize: 12 }}>{h.goal}</div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Message input bar (visible when team exists and on messages tab) */}
      {team && tab === "messages" && (
        <div style={{
          padding: "8px 12px", borderTop: "1px solid var(--border-color)",
          display: "flex", gap: 8, alignItems: "center",
        }}>
          <input
            value={userMsg}
            onChange={(e) => setUserMsg(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleSendMessage()}
            placeholder="Send a message to the team..."
            style={{ ...inputStyle, flex: 1 }}
          />
          <button onClick={handleSendMessage} disabled={!userMsg.trim()} style={{
            ...btnPrimary,
            opacity: !userMsg.trim() ? 0.5 : 1,
            cursor: !userMsg.trim() ? "not-allowed" : "pointer",
          }}>
            Send
          </button>
        </div>
      )}
    </div>
  );
};

export default AgentTeamsPanel;

const btnPrimary: React.CSSProperties = {
  padding: "6px 14px", fontSize: 12, fontWeight: 600,
  border: "none", borderRadius: 4,
  background: "var(--accent-color)", color: "white",
  cursor: "pointer",
};

const btnSecondary: React.CSSProperties = {
  padding: "4px 10px", fontSize: 10, fontWeight: 600,
  border: "1px solid var(--border-color)", borderRadius: 4,
  background: "var(--bg-secondary)", color: "var(--text-primary)",
  cursor: "pointer",
};

const inputStyle: React.CSSProperties = {
  padding: "6px 10px", fontSize: 12, borderRadius: 4,
  border: "1px solid var(--border-color)",
  background: "var(--bg-primary)", color: "var(--text-primary)",
  outline: "none",
};

const labelStyle: React.CSSProperties = {
  fontSize: 10, fontWeight: 600, marginBottom: 4,
  color: "var(--text-muted)",
  textTransform: "uppercase" as const, letterSpacing: "0.05em",
};
