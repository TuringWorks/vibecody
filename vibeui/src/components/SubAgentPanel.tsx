import React, { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// -- Types --------------------------------------------------------------------

type TabName = "Agents" | "Results" | "Spawn";

interface SubAgentDto {
  id: string;
  role: string;
  status: string;
  context_files: string[];
  provider: string;
  task_description: string | null;
  result_summary: string | null;
  findings: string[];
  files_modified: string[];
  created_at: string;
  completed_at: string | null;
  error: string | null;
}

interface SubAgentPanelProps {
  provider: string;
}

const ROLES = [
  { name: "Oracle", description: "General-purpose reasoning agent that answers complex questions about the codebase." },
  { name: "Librarian", description: "Indexes and retrieves relevant code, docs, and context files for other agents." },
  { name: "Implementer", description: "Writes new code or modifies existing files to fulfill a task." },
  { name: "Reviewer", description: "Reviews code changes for quality, patterns, and best practices." },
  { name: "Tester", description: "Generates unit and integration tests for specified code." },
  { name: "Documenter", description: "Generates and updates documentation for public APIs and modules." },
  { name: "Architect", description: "Analyzes system design and proposes architectural improvements." },
  { name: "Debugger", description: "Investigates bugs, traces root causes, and suggests fixes." },
  { name: "Optimizer", description: "Profiles and optimizes code for performance and resource usage." },
  { name: "SecurityExpert", description: "Scans code for security vulnerabilities and misconfigurations." },
] as const;

// -- Helpers ------------------------------------------------------------------

const statusColor = (s: string): string => {
  switch (s) {
    case "working": return "var(--warning-color)";
    case "completed": return "var(--success-color)";
    case "failed": return "var(--error-color)";
    default: return "var(--text-muted)";
  }
};

const statusLabel = (s: string): string => {
  switch (s) {
    case "working": return "Working";
    case "completed": return "Completed";
    case "failed": return "Failed";
    default: return s;
  }
};

const formatTimestamp = (ts: string): string => {
  try {
    const d = new Date(ts);
    return d.toLocaleString();
  } catch {
    return ts;
  }
};

// -- Styles -------------------------------------------------------------------

const containerStyle: React.CSSProperties = {
  padding: 12,
  fontFamily: "inherit",
  fontSize: 13,
  height: "100%",
  overflowY: "auto",
  color: "var(--text-secondary)",
  background: "var(--bg-primary)",
};

const tabBarStyle: React.CSSProperties = {
  display: "flex",
  gap: 0,
  marginBottom: 12,
  borderBottom: "1px solid var(--border-color)",
};

const cardStyle: React.CSSProperties = {
  padding: "10px 12px",
  marginBottom: 8,
  borderRadius: 4,
  background: "var(--bg-secondary)",
};

const badgeStyle = (color: string): React.CSSProperties => ({
  fontSize: 10,
  padding: "2px 8px",
  borderRadius: 10,
  background: color,
  color: "white",
  fontWeight: 600,
});

const inputStyle: React.CSSProperties = {
  width: "100%",
  padding: "6px 8px",
  fontSize: 12,
  borderRadius: 4,
  border: "1px solid var(--border-color)",
  background: "var(--bg-secondary)",
  color: "var(--text-primary)",
  fontFamily: "inherit",
  boxSizing: "border-box",
};

const buttonStyle: React.CSSProperties = {
  padding: "6px 16px",
  fontSize: 12,
  borderRadius: 4,
  border: "none",
  cursor: "pointer",
  fontWeight: 600,
};

// -- Component ----------------------------------------------------------------

const SubAgentPanel: React.FC<SubAgentPanelProps> = ({ provider }) => {
  const [tab, setTab] = useState<TabName>("Agents");
  const [agents, setAgents] = useState<SubAgentDto[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [expandedAgent, setExpandedAgent] = useState<string | null>(null);

  // Spawn form state
  const [spawnRole, setSpawnRole] = useState<string>(ROLES[0].name);
  const [spawnTask, setSpawnTask] = useState("");
  const [spawnContextFiles, setSpawnContextFiles] = useState("");
  const [spawnProvider, setSpawnProvider] = useState(provider);
  const [spawning, setSpawning] = useState(false);
  const [spawnError, setSpawnError] = useState<string | null>(null);
  const [spawnSuccess, setSpawnSuccess] = useState<string | null>(null);

  const tabs: TabName[] = ["Agents", "Results", "Spawn"];

  const fetchAgents = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<SubAgentDto[]>("list_sub_agents");
      setAgents(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchAgents();
  }, [fetchAgents]);

  useEffect(() => {
    const unlisten = listen("subagent:updated", () => {
      fetchAgents();
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [fetchAgents]);

  const handleDismiss = async (agentId: string) => {
    try {
      await invoke("dismiss_sub_agent", { agentId });
      setAgents((prev) => prev.filter((a) => a.id !== agentId));
    } catch (err) {
      setError(String(err));
    }
  };

  const handleClearCompleted = async () => {
    try {
      await invoke("clear_completed_sub_agents");
      setAgents((prev) => prev.filter((a) => a.status === "working"));
    } catch (err) {
      setError(String(err));
    }
  };

  const handleSpawn = async () => {
    if (!spawnTask.trim()) {
      setSpawnError("Task description is required.");
      return;
    }
    setSpawning(true);
    setSpawnError(null);
    setSpawnSuccess(null);
    try {
      const contextFiles = spawnContextFiles
        .split(/[,\n]/)
        .map((f) => f.trim())
        .filter((f) => f.length > 0);
      const newAgent = await invoke<SubAgentDto>("spawn_sub_agent", {
        role: spawnRole,
        task: spawnTask.trim(),
        contextFiles,
        provider: spawnProvider.trim() || provider,
      });
      setAgents((prev) => [newAgent, ...prev]);
      setSpawnSuccess(`Agent "${newAgent.role}" spawned (${newAgent.id})`);
      setSpawnTask("");
      setSpawnContextFiles("");
    } catch (err) {
      setSpawnError(String(err));
    } finally {
      setSpawning(false);
    }
  };

  const completedOrFailed = agents.filter((a) => a.status === "completed" || a.status === "failed");
  const hasCompleted = completedOrFailed.length > 0;
  const selectedRoleDesc = ROLES.find((r) => r.name === spawnRole)?.description ?? "";

  return (
    <div style={containerStyle}>
      <div style={{ fontWeight: 600, fontSize: 14, marginBottom: 12, color: "var(--text-primary)" }}>Sub-Agents</div>

      {/* Tab bar */}
      <div style={tabBarStyle}>
        {tabs.map((t) => (
          <button
            key={t}
            onClick={() => setTab(t)}
            style={{
              padding: "6px 16px",
              fontSize: 12,
              background: "none",
              border: "none",
              borderBottom: tab === t ? "2px solid var(--accent-color)" : "2px solid transparent",
              color: tab === t ? "var(--text-primary)" : "var(--text-muted)",
              cursor: "pointer",
              fontWeight: tab === t ? 600 : 400,
            }}
          >
            {t}
          </button>
        ))}
      </div>

      {/* Global error */}
      {error && (
        <div style={{ padding: "8px 10px", marginBottom: 8, borderRadius: 4, background: "var(--error-color)", color: "white", fontSize: 12 }}>
          {error}
        </div>
      )}

      {/* Agents Tab */}
      {tab === "Agents" && (
        <div>
          {/* Clear completed button */}
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
            <span style={{ fontSize: 11, color: "var(--text-muted)" }}>
              {agents.length} agent{agents.length !== 1 ? "s" : ""}
              {loading ? " (refreshing...)" : ""}
            </span>
            {hasCompleted && (
              <button
                onClick={handleClearCompleted}
                style={{ ...buttonStyle, background: "var(--border-color)", color: "var(--text-primary)", fontSize: 11, padding: "4px 10px" }}
              >
                Clear completed
              </button>
            )}
          </div>

          {agents.length === 0 && !loading && (
            <div style={{ padding: 20, textAlign: "center", color: "var(--text-muted)", fontSize: 12 }}>
              No sub-agents running. Go to the Spawn tab to create one.
            </div>
          )}

          {agents.map((agent) => (
            <div
              key={agent.id}
              onClick={() => setExpandedAgent(expandedAgent === agent.id ? null : agent.id)}
              style={{ ...cardStyle, borderLeft: `3px solid ${statusColor(agent.status)}`, cursor: "pointer" }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span style={{ fontWeight: 600, fontSize: 12, color: "var(--text-primary)" }}>{agent.role}</span>
                <span style={badgeStyle(statusColor(agent.status))}>{statusLabel(agent.status)}</span>
                <span style={{ marginLeft: "auto", fontSize: 10, color: "var(--text-muted)" }}>{agent.provider}</span>
              </div>

              {agent.task_description && (
                <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 4, lineHeight: 1.4 }}>
                  {agent.task_description}
                </div>
              )}

              <div style={{ display: "flex", gap: 12, marginTop: 4, fontSize: 10, color: "var(--text-muted)" }}>
                <span>Created: {formatTimestamp(agent.created_at)}</span>
                {agent.completed_at && <span>Completed: {formatTimestamp(agent.completed_at)}</span>}
              </div>

              {expandedAgent === agent.id && (
                <div style={{ marginTop: 8 }}>
                  <div style={{ fontSize: 10, color: "var(--text-muted)", marginBottom: 2 }}>ID: {agent.id}</div>

                  {agent.context_files.length > 0 && (
                    <>
                      <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 6, marginBottom: 4 }}>Context Files:</div>
                      {agent.context_files.map((f) => (
                        <div key={f} style={{ fontSize: 11, fontFamily: "monospace", padding: "2px 6px", marginBottom: 2, background: "var(--bg-primary)", borderRadius: 3 }}>{f}</div>
                      ))}
                    </>
                  )}

                  {agent.error && (
                    <div style={{ fontSize: 11, color: "var(--error-color)", marginTop: 6 }}>
                      Error: {agent.error}
                    </div>
                  )}

                  {(agent.status === "completed" || agent.status === "failed") && (
                    <button
                      onClick={(e) => { e.stopPropagation(); handleDismiss(agent.id); }}
                      style={{ ...buttonStyle, marginTop: 8, background: "var(--border-color)", color: "var(--text-primary)", fontSize: 11, padding: "4px 10px" }}
                    >
                      Dismiss
                    </button>
                  )}
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {/* Results Tab */}
      {tab === "Results" && (
        <div>
          {completedOrFailed.length === 0 && (
            <div style={{ padding: 20, textAlign: "center", color: "var(--text-muted)", fontSize: 12 }}>
              No completed or failed agents yet.
            </div>
          )}

          {completedOrFailed.map((agent) => {
            const isSuccess = agent.status === "completed";
            const borderColor = isSuccess ? "var(--success-color)" : "var(--error-color)";

            return (
              <div key={agent.id} style={{ ...cardStyle, borderLeft: `3px solid ${borderColor}` }}>
                <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                  <span style={{ fontWeight: 600, fontSize: 12, color: "var(--text-primary)" }}>{agent.role}</span>
                  <span style={badgeStyle(borderColor)}>{isSuccess ? "Completed" : "Failed"}</span>
                  <span style={{ marginLeft: "auto", fontSize: 10, color: "var(--text-muted)" }}>
                    {agent.completed_at ? formatTimestamp(agent.completed_at) : ""}
                  </span>
                </div>

                {agent.result_summary && (
                  <div style={{ fontSize: 12, marginBottom: 6, lineHeight: 1.5, color: "var(--text-secondary)" }}>
                    {agent.result_summary}
                  </div>
                )}

                {agent.error && (
                  <div style={{ fontSize: 12, marginBottom: 6, lineHeight: 1.5, color: "var(--error-color)" }}>
                    {agent.error}
                  </div>
                )}

                {agent.findings.length > 0 && (
                  <>
                    <div style={{ fontSize: 11, color: "var(--text-muted)", marginBottom: 4 }}>Findings:</div>
                    <ul style={{ margin: 0, paddingLeft: 16, fontSize: 11, lineHeight: 1.6, color: "var(--text-secondary)" }}>
                      {agent.findings.map((f, i) => (
                        <li key={i}>{f}</li>
                      ))}
                    </ul>
                  </>
                )}

                {agent.files_modified.length > 0 && (
                  <div style={{ marginTop: 6 }}>
                    <span style={{ fontSize: 11, color: "var(--text-muted)" }}>Modified: </span>
                    {agent.files_modified.map((f) => (
                      <span key={f} style={{ fontSize: 10, fontFamily: "monospace", padding: "1px 5px", borderRadius: 3, background: "var(--bg-primary)", marginLeft: 4 }}>{f}</span>
                    ))}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}

      {/* Spawn Tab */}
      {tab === "Spawn" && (
        <div>
          {spawnError && (
            <div style={{ padding: "8px 10px", marginBottom: 8, borderRadius: 4, background: "var(--error-color)", color: "white", fontSize: 12 }}>
              {spawnError}
            </div>
          )}
          {spawnSuccess && (
            <div style={{ padding: "8px 10px", marginBottom: 8, borderRadius: 4, background: "var(--success-color)", color: "white", fontSize: 12 }}>
              {spawnSuccess}
            </div>
          )}

          {/* Role */}
          <div style={{ marginBottom: 10 }}>
            <label style={{ fontSize: 11, color: "var(--text-muted)", display: "block", marginBottom: 4 }}>Role</label>
            <select
              value={spawnRole}
              onChange={(e) => setSpawnRole(e.target.value)}
              style={{ ...inputStyle, cursor: "pointer" }}
            >
              {ROLES.map((r) => (
                <option key={r.name} value={r.name}>{r.name}</option>
              ))}
            </select>
            {selectedRoleDesc && (
              <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 4, fontStyle: "italic" }}>
                {selectedRoleDesc}
              </div>
            )}
          </div>

          {/* Task */}
          <div style={{ marginBottom: 10 }}>
            <label style={{ fontSize: 11, color: "var(--text-muted)", display: "block", marginBottom: 4 }}>Task Description</label>
            <textarea
              value={spawnTask}
              onChange={(e) => setSpawnTask(e.target.value)}
              placeholder="Describe the task for the sub-agent..."
              rows={4}
              style={{ ...inputStyle, resize: "vertical" }}
            />
          </div>

          {/* Context files */}
          <div style={{ marginBottom: 10 }}>
            <label style={{ fontSize: 11, color: "var(--text-muted)", display: "block", marginBottom: 4 }}>Context Files (comma-separated or one per line)</label>
            <textarea
              value={spawnContextFiles}
              onChange={(e) => setSpawnContextFiles(e.target.value)}
              placeholder="src/main.rs, src/lib.rs"
              rows={3}
              style={{ ...inputStyle, resize: "vertical", fontFamily: "monospace" }}
            />
          </div>

          {/* Provider */}
          <div style={{ marginBottom: 14 }}>
            <label style={{ fontSize: 11, color: "var(--text-muted)", display: "block", marginBottom: 4 }}>Provider</label>
            <input
              type="text"
              value={spawnProvider}
              onChange={(e) => setSpawnProvider(e.target.value)}
              style={inputStyle}
            />
          </div>

          {/* Spawn button */}
          <button
            onClick={handleSpawn}
            disabled={spawning || !spawnTask.trim()}
            style={{
              ...buttonStyle,
              background: spawning || !spawnTask.trim() ? "var(--border-color)" : "var(--accent-color)",
              color: "white",
              width: "100%",
              padding: "8px 16px",
              opacity: spawning || !spawnTask.trim() ? 0.6 : 1,
            }}
          >
            {spawning ? "Spawning..." : "Spawn Agent"}
          </button>
        </div>
      )}
    </div>
  );
};

export default SubAgentPanel;
