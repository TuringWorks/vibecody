/**
 * A2aPanel -- A2A (Agent-to-Agent) Protocol panel.
 *
 * Discover remote A2A agents, submit tasks, manage VibeCody's own agent card,
 * and monitor protocol metrics.
 * Wired to Tauri backend commands for persistent state.
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

// ── Types ─────────────────────────────────────────────────────────────────────

interface A2aAgent {
  id: string;
  name: string;
  url: string;
  capabilities: string[];
  status: "online" | "offline" | "unknown";
}

interface A2aTask {
  id: string;
  agent_url: string;
  agent_name: string;
  input: string;
  content_type: string;
  status: "submitted" | "working" | "completed" | "failed" | "cancelled";
  created_at: string;
  completed_at: string | null;
}

interface A2aAgentCard {
  name: string;
  description: string;
  url: string;
  version: string;
  capabilities: string[];
}

interface A2aMetrics {
  tasks_created: number;
  tasks_completed: number;
  tasks_failed: number;
  tasks_cancelled: number;
  success_rate: number;
  avg_completion_time_ms: number;
  agents_discovered: number;
}

// ── All possible capabilities ─────────────────────────────────────────────────

const ALL_CAPABILITIES = [
  "CodeGeneration",
  "CodeReview",
  "Testing",
  "Debugging",
  "Refactoring",
  "Documentation",
  "Security",
  "Deployment",
  "DataAnalysis",
] as const;

// ── Styles ────────────────────────────────────────────────────────────────────

const panelStyle: React.CSSProperties = {
  padding: 16,
  height: "100%",
  overflow: "auto",
  color: "var(--text-primary)",
  background: "var(--bg-primary)",
  fontFamily: "var(--font-family)",
  fontSize: 13,
};

const headingStyle: React.CSSProperties = {
  margin: "0 0 12px",
  fontSize: 18,
  fontWeight: 600,
  color: "var(--text-primary)",
};

const cardStyle: React.CSSProperties = {
  background: "var(--bg-secondary)",
  borderRadius: "var(--border-radius, 6px)",
  padding: 12,
  marginBottom: 10,
  border: "1px solid var(--border)",
};

const inputStyle: React.CSSProperties = {
  width: "100%",
  padding: "6px 10px",
  borderRadius: "var(--border-radius, 4px)",
  border: "1px solid var(--border)",
  background: "var(--bg-tertiary)",
  color: "var(--text-primary)",
  fontSize: 13,
  boxSizing: "border-box",
};

const textareaStyle: React.CSSProperties = {
  ...inputStyle,
  minHeight: 80,
  resize: "vertical",
  fontFamily: "monospace",
};

const selectStyle: React.CSSProperties = {
  ...inputStyle,
  cursor: "pointer",
};

const btnStyle: React.CSSProperties = {
  padding: "6px 14px",
  borderRadius: "var(--border-radius, 4px)",
  border: "none",
  background: "var(--accent)",
  color: "#fff",
  cursor: "pointer",
  fontSize: 13,
  fontWeight: 500,
};

const btnDangerStyle: React.CSSProperties = {
  ...btnStyle,
  background: "var(--error)",
};

const btnSecondaryStyle: React.CSSProperties = {
  ...btnStyle,
  background: "var(--bg-tertiary)",
  color: "var(--text-primary)",
  border: "1px solid var(--border)",
};

const tabBarStyle: React.CSSProperties = {
  display: "flex",
  gap: 0,
  borderBottom: "1px solid var(--border)",
  marginBottom: 16,
};

const tabBtnStyle = (active: boolean): React.CSSProperties => ({
  padding: "8px 16px",
  cursor: "pointer",
  borderBottom: active ? "2px solid var(--accent)" : "2px solid transparent",
  color: active ? "var(--accent)" : "var(--text-secondary)",
  background: "transparent",
  border: "none",
  borderBottomStyle: "solid",
  borderBottomWidth: 2,
  borderBottomColor: active ? "var(--accent)" : "transparent",
  fontSize: 13,
  fontWeight: active ? 600 : 400,
});

const labelStyle: React.CSSProperties = {
  fontSize: 11,
  color: "var(--text-secondary)",
  marginBottom: 4,
  display: "block",
};

const capBadgeColors: Record<string, string> = {
  CodeGeneration: "#6366f1",
  CodeReview: "#8b5cf6",
  Testing: "#22c55e",
  Debugging: "#f59e0b",
  Refactoring: "#06b6d4",
  Documentation: "#3b82f6",
  Security: "#ef4444",
  Deployment: "#f97316",
  DataAnalysis: "#ec4899",
};

const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block",
  padding: "2px 8px",
  borderRadius: 10,
  fontSize: 11,
  fontWeight: 600,
  background: color,
  color: "#fff",
  marginRight: 4,
  marginBottom: 4,
});

const statusBadgeColors: Record<string, string> = {
  online: "var(--success, #22c55e)",
  offline: "#6b7280",
  unknown: "var(--warning, #f59e0b)",
  submitted: "#6366f1",
  working: "var(--warning, #f59e0b)",
  completed: "var(--success, #22c55e)",
  failed: "var(--error, #ef4444)",
  cancelled: "#6b7280",
};

const metricCardStyle: React.CSSProperties = {
  ...cardStyle,
  textAlign: "center",
};

const errorBannerStyle: React.CSSProperties = {
  background: "var(--error, #ef4444)",
  color: "#fff",
  padding: "8px 12px",
  borderRadius: "var(--border-radius, 4px)",
  marginBottom: 12,
  fontSize: 12,
};

const rowStyle: React.CSSProperties = {
  display: "flex",
  gap: 8,
  alignItems: "center",
  marginBottom: 8,
};

// ── Component ─────────────────────────────────────────────────────────────────

type Tab = "agents" | "tasks" | "card" | "metrics";

export function A2aPanel() {
  const [tab, setTab] = useState<Tab>("agents");
  const [error, setError] = useState<string | null>(null);

  // ── Agents tab state ────────────────────────────────────────────────────
  const [agents, setAgents] = useState<A2aAgent[]>([]);
  const [agentsLoading, setAgentsLoading] = useState(false);
  const [discoverUrl, setDiscoverUrl] = useState("");

  // ── Tasks tab state ─────────────────────────────────────────────────────
  const [tasks, setTasks] = useState<A2aTask[]>([]);
  const [tasksLoading, setTasksLoading] = useState(false);
  const [taskAgentUrl, setTaskAgentUrl] = useState("");
  const [taskInput, setTaskInput] = useState("");
  const [taskContentType, setTaskContentType] = useState("text");
  const [submitting, setSubmitting] = useState(false);

  // ── Agent Card tab state ────────────────────────────────────────────────
  const [agentCard, setAgentCard] = useState<A2aAgentCard>({
    name: "",
    description: "",
    url: "",
    version: "",
    capabilities: [],
  });
  const [cardLoading, setCardLoading] = useState(false);
  const [cardSaving, setCardSaving] = useState(false);

  // ── Metrics tab state ───────────────────────────────────────────────────
  const [metrics, setMetrics] = useState<A2aMetrics | null>(null);
  const [metricsLoading, setMetricsLoading] = useState(false);

  // ── Data fetchers ───────────────────────────────────────────────────────

  const fetchAgents = useCallback(async () => {
    setAgentsLoading(true);
    try {
      const res = await invoke<A2aAgent[]>("a2a_list_agents");
      setAgents(res);
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setAgentsLoading(false);
    }
  }, []);

  const fetchTasks = useCallback(async () => {
    setTasksLoading(true);
    try {
      const res = await invoke<A2aTask[]>("a2a_list_tasks");
      setTasks(res);
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setTasksLoading(false);
    }
  }, []);

  const fetchAgentCard = useCallback(async () => {
    setCardLoading(true);
    try {
      const res = await invoke<A2aAgentCard>("a2a_get_agent_card");
      setAgentCard(res);
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setCardLoading(false);
    }
  }, []);

  const fetchMetrics = useCallback(async () => {
    setMetricsLoading(true);
    try {
      const res = await invoke<A2aMetrics>("a2a_get_metrics");
      setMetrics(res);
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setMetricsLoading(false);
    }
  }, []);

  // ── Load data on tab change ─────────────────────────────────────────────

  useEffect(() => {
    if (tab === "agents") fetchAgents();
    if (tab === "tasks") fetchTasks();
    if (tab === "card") fetchAgentCard();
    if (tab === "metrics") fetchMetrics();
  }, [tab, fetchAgents, fetchTasks, fetchAgentCard, fetchMetrics]);

  // ── Metrics auto-refresh (5s) ───────────────────────────────────────────

  useEffect(() => {
    if (tab !== "metrics") return;
    const interval = setInterval(fetchMetrics, 5000);
    return () => clearInterval(interval);
  }, [tab, fetchMetrics]);

  // ── Handlers ────────────────────────────────────────────────────────────

  const handleDiscover = useCallback(async () => {
    if (!discoverUrl.trim()) return;
    try {
      await invoke("a2a_discover", { url: discoverUrl.trim() });
      setDiscoverUrl("");
      setError(null);
      await fetchAgents();
    } catch (e) {
      setError(String(e));
    }
  }, [discoverUrl, fetchAgents]);

  const handleSubmitTask = useCallback(async () => {
    if (!taskAgentUrl || !taskInput.trim()) return;
    setSubmitting(true);
    try {
      await invoke("a2a_submit_task", {
        agentUrl: taskAgentUrl,
        input: taskInput.trim(),
        contentType: taskContentType,
      });
      setTaskInput("");
      setError(null);
      await fetchTasks();
    } catch (e) {
      setError(String(e));
    } finally {
      setSubmitting(false);
    }
  }, [taskAgentUrl, taskInput, taskContentType, fetchTasks]);

  const handleCancelTask = useCallback(
    async (taskId: string) => {
      try {
        await invoke("a2a_cancel_task", { taskId });
        setError(null);
        await fetchTasks();
      } catch (e) {
        setError(String(e));
      }
    },
    [fetchTasks]
  );

  const handleSaveCard = useCallback(async () => {
    setCardSaving(true);
    try {
      await invoke("a2a_update_agent_card", {
        name: agentCard.name,
        description: agentCard.description,
        capabilities: agentCard.capabilities,
      });
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setCardSaving(false);
    }
  }, [agentCard]);

  const toggleCapability = useCallback((cap: string) => {
    setAgentCard((prev) => ({
      ...prev,
      capabilities: prev.capabilities.includes(cap)
        ? prev.capabilities.filter((c) => c !== cap)
        : [...prev.capabilities, cap],
    }));
  }, []);

  const isTerminalStatus = (status: string) =>
    status === "completed" || status === "failed" || status === "cancelled";

  const formatTime = (iso: string) => {
    try {
      return new Date(iso).toLocaleString();
    } catch {
      return iso;
    }
  };

  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`;
    if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
    return `${(ms / 60000).toFixed(1)}m`;
  };

  // ── Render ──────────────────────────────────────────────────────────────

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>A2A Protocol</h2>

      {error && (
        <div style={errorBannerStyle}>
          {error}
          <button
            style={{ marginLeft: 8, background: "transparent", border: "none", color: "#fff", cursor: "pointer", fontWeight: 600 }}
            onClick={() => setError(null)}
          >
            Dismiss
          </button>
        </div>
      )}

      <div style={tabBarStyle}>
        <button style={tabBtnStyle(tab === "agents")} onClick={() => setTab("agents")}>Agents</button>
        <button style={tabBtnStyle(tab === "tasks")} onClick={() => setTab("tasks")}>Tasks</button>
        <button style={tabBtnStyle(tab === "card")} onClick={() => setTab("card")}>Agent Card</button>
        <button style={tabBtnStyle(tab === "metrics")} onClick={() => setTab("metrics")}>Metrics</button>
      </div>

      {/* ── Agents Tab ─────────────────────────────────────────────────────── */}
      {tab === "agents" && (
        <div>
          <div style={{ ...cardStyle, display: "flex", gap: 8, alignItems: "center" }}>
            <input
              style={{ ...inputStyle, flex: 1 }}
              placeholder="Agent URL (e.g. http://localhost:9100/.well-known/agent.json)"
              value={discoverUrl}
              onChange={(e) => setDiscoverUrl(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleDiscover()}
            />
            <button style={btnStyle} onClick={handleDiscover}>
              Discover Agent
            </button>
            <button style={btnSecondaryStyle} onClick={fetchAgents}>
              Refresh
            </button>
          </div>

          {agentsLoading && <div style={{ color: "var(--text-muted)", padding: 12 }}>Loading agents...</div>}

          {!agentsLoading && agents.length === 0 && (
            <div style={{ color: "var(--text-muted)", padding: 24, textAlign: "center" }}>
              No agents discovered yet. Enter a URL above to discover an A2A agent.
            </div>
          )}

          {agents.map((agent) => (
            <div key={agent.id} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <strong style={{ fontSize: 14 }}>{agent.name}</strong>
                <span style={badgeStyle(statusBadgeColors[agent.status] || "#6b7280")}>
                  {agent.status}
                </span>
              </div>
              <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 4, fontFamily: "monospace" }}>
                {agent.url}
              </div>
              {agent.capabilities.length > 0 && (
                <div style={{ marginTop: 8 }}>
                  {agent.capabilities.map((cap) => (
                    <span key={cap} style={badgeStyle(capBadgeColors[cap] || "#6366f1")}>
                      {cap}
                    </span>
                  ))}
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {/* ── Tasks Tab ──────────────────────────────────────────────────────── */}
      {tab === "tasks" && (
        <div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, marginBottom: 8 }}>Submit Task</div>

            <div style={rowStyle}>
              <label style={{ ...labelStyle, marginBottom: 0, minWidth: 50 }}>Agent:</label>
              <select
                style={{ ...selectStyle, flex: 1 }}
                value={taskAgentUrl}
                onChange={(e) => setTaskAgentUrl(e.target.value)}
              >
                <option value="">-- Select an agent --</option>
                {agents.map((a) => (
                  <option key={a.id} value={a.url}>
                    {a.name} ({a.url})
                  </option>
                ))}
              </select>
            </div>

            <div style={{ marginBottom: 8 }}>
              <label style={labelStyle}>Content Type:</label>
              <div style={{ display: "flex", gap: 8 }}>
                {["text", "code", "json"].map((ct) => (
                  <button
                    key={ct}
                    style={{
                      ...btnSecondaryStyle,
                      background: taskContentType === ct ? "var(--accent)" : "var(--bg-tertiary)",
                      color: taskContentType === ct ? "#fff" : "var(--text-primary)",
                    }}
                    onClick={() => setTaskContentType(ct)}
                  >
                    {ct}
                  </button>
                ))}
              </div>
            </div>

            <div style={{ marginBottom: 8 }}>
              <label style={labelStyle}>Input:</label>
              <textarea
                style={textareaStyle}
                placeholder="Enter task input..."
                value={taskInput}
                onChange={(e) => setTaskInput(e.target.value)}
              />
            </div>

            <button
              style={{ ...btnStyle, opacity: submitting ? 0.6 : 1 }}
              onClick={handleSubmitTask}
              disabled={submitting || !taskAgentUrl || !taskInput.trim()}
            >
              {submitting ? "Submitting..." : "Submit Task"}
            </button>
          </div>

          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", margin: "12px 0 8px" }}>
            <div style={{ fontWeight: 600 }}>Task History</div>
            <button style={btnSecondaryStyle} onClick={fetchTasks}>
              Refresh
            </button>
          </div>

          {tasksLoading && <div style={{ color: "var(--text-muted)", padding: 12 }}>Loading tasks...</div>}

          {!tasksLoading && tasks.length === 0 && (
            <div style={{ color: "var(--text-muted)", padding: 24, textAlign: "center" }}>
              No tasks yet. Submit a task to an agent above.
            </div>
          )}

          {tasks.map((task) => (
            <div key={task.id} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <div style={{ flex: 1 }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    <span style={{ fontFamily: "monospace", fontSize: 11, color: "var(--text-muted)" }}>
                      {task.id.slice(0, 8)}
                    </span>
                    <span style={badgeStyle(statusBadgeColors[task.status] || "#6b7280")}>
                      {task.status}
                    </span>
                  </div>
                  <div style={{ marginTop: 4, fontSize: 12, color: "var(--text-secondary)" }}>
                    Agent: <strong>{task.agent_name || task.agent_url}</strong>
                  </div>
                </div>
                {!isTerminalStatus(task.status) && (
                  <button style={btnDangerStyle} onClick={() => handleCancelTask(task.id)}>
                    Cancel
                  </button>
                )}
              </div>
              <div
                style={{
                  marginTop: 8,
                  padding: 8,
                  background: "var(--bg-tertiary)",
                  borderRadius: "var(--border-radius, 4px)",
                  fontSize: 12,
                  fontFamily: "monospace",
                  whiteSpace: "pre-wrap",
                  maxHeight: 120,
                  overflow: "auto",
                }}
              >
                {task.input}
              </div>
              <div style={{ marginTop: 6, fontSize: 11, color: "var(--text-muted)" }}>
                Created: {formatTime(task.created_at)}
                {task.completed_at && <> | Completed: {formatTime(task.completed_at)}</>}
              </div>
            </div>
          ))}
        </div>
      )}

      {/* ── Agent Card Tab ─────────────────────────────────────────────────── */}
      {tab === "card" && (
        <div>
          {cardLoading ? (
            <div style={{ color: "var(--text-muted)", padding: 12 }}>Loading agent card...</div>
          ) : (
            <div style={cardStyle}>
              <div style={{ fontWeight: 600, marginBottom: 12 }}>VibeCody Agent Card</div>

              <div style={{ marginBottom: 10 }}>
                <label style={labelStyle}>Name</label>
                <input
                  style={inputStyle}
                  value={agentCard.name}
                  onChange={(e) => setAgentCard((prev) => ({ ...prev, name: e.target.value }))}
                  placeholder="VibeCody"
                />
              </div>

              <div style={{ marginBottom: 10 }}>
                <label style={labelStyle}>Description</label>
                <textarea
                  style={{ ...textareaStyle, minHeight: 60 }}
                  value={agentCard.description}
                  onChange={(e) => setAgentCard((prev) => ({ ...prev, description: e.target.value }))}
                  placeholder="AI-powered code assistant with multi-provider support"
                />
              </div>

              <div style={{ marginBottom: 10 }}>
                <label style={labelStyle}>URL</label>
                <input
                  style={{ ...inputStyle, color: "var(--text-muted)" }}
                  value={agentCard.url}
                  readOnly
                  title="URL is set by the server configuration"
                />
              </div>

              <div style={{ marginBottom: 10 }}>
                <label style={labelStyle}>Version</label>
                <input
                  style={{ ...inputStyle, color: "var(--text-muted)" }}
                  value={agentCard.version}
                  readOnly
                  title="Version is set automatically"
                />
              </div>

              <div style={{ marginBottom: 12 }}>
                <label style={labelStyle}>Capabilities</label>
                <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
                  {ALL_CAPABILITIES.map((cap) => {
                    const checked = agentCard.capabilities.includes(cap);
                    return (
                      <label
                        key={cap}
                        style={{
                          display: "flex",
                          alignItems: "center",
                          gap: 4,
                          padding: "4px 10px",
                          borderRadius: "var(--border-radius, 4px)",
                          background: checked ? (capBadgeColors[cap] || "var(--accent)") : "var(--bg-tertiary)",
                          color: checked ? "#fff" : "var(--text-secondary)",
                          cursor: "pointer",
                          fontSize: 12,
                          fontWeight: checked ? 600 : 400,
                          border: `1px solid ${checked ? "transparent" : "var(--border)"}`,
                          transition: "all 0.15s ease",
                        }}
                      >
                        <input
                          type="checkbox"
                          checked={checked}
                          onChange={() => toggleCapability(cap)}
                          style={{ display: "none" }}
                        />
                        {cap}
                      </label>
                    );
                  })}
                </div>
              </div>

              <button
                style={{ ...btnStyle, opacity: cardSaving ? 0.6 : 1 }}
                onClick={handleSaveCard}
                disabled={cardSaving}
              >
                {cardSaving ? "Saving..." : "Save Agent Card"}
              </button>
            </div>
          )}
        </div>
      )}

      {/* ── Metrics Tab ────────────────────────────────────────────────────── */}
      {tab === "metrics" && (
        <div>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
            <div style={{ fontSize: 12, color: "var(--text-muted)" }}>Auto-refreshes every 5 seconds</div>
            <button style={btnSecondaryStyle} onClick={fetchMetrics}>
              Refresh
            </button>
          </div>

          {metricsLoading && !metrics && (
            <div style={{ color: "var(--text-muted)", padding: 12 }}>Loading metrics...</div>
          )}

          {metrics && (
            <>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 10, marginBottom: 16 }}>
                <div style={metricCardStyle}>
                  <div style={labelStyle}>Tasks Created</div>
                  <div style={{ fontSize: 28, fontWeight: 700, color: "var(--text-primary)" }}>
                    {metrics.tasks_created}
                  </div>
                </div>
                <div style={metricCardStyle}>
                  <div style={labelStyle}>Completed</div>
                  <div style={{ fontSize: 28, fontWeight: 700, color: "var(--success, #22c55e)" }}>
                    {metrics.tasks_completed}
                  </div>
                </div>
                <div style={metricCardStyle}>
                  <div style={labelStyle}>Failed</div>
                  <div style={{ fontSize: 28, fontWeight: 700, color: "var(--error, #ef4444)" }}>
                    {metrics.tasks_failed}
                  </div>
                </div>
              </div>

              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 10 }}>
                <div style={metricCardStyle}>
                  <div style={labelStyle}>Cancelled</div>
                  <div style={{ fontSize: 28, fontWeight: 700, color: "var(--text-muted)" }}>
                    {metrics.tasks_cancelled}
                  </div>
                </div>
                <div style={metricCardStyle}>
                  <div style={labelStyle}>Success Rate</div>
                  <div
                    style={{
                      fontSize: 28,
                      fontWeight: 700,
                      color:
                        metrics.success_rate >= 80
                          ? "var(--success, #22c55e)"
                          : metrics.success_rate >= 50
                          ? "var(--warning, #f59e0b)"
                          : "var(--error, #ef4444)",
                    }}
                  >
                    {metrics.success_rate.toFixed(1)}%
                  </div>
                </div>
                <div style={metricCardStyle}>
                  <div style={labelStyle}>Avg Completion Time</div>
                  <div style={{ fontSize: 28, fontWeight: 700, color: "var(--accent)" }}>
                    {formatDuration(metrics.avg_completion_time_ms)}
                  </div>
                </div>
              </div>

              <div style={{ ...metricCardStyle, marginTop: 10 }}>
                <div style={labelStyle}>Agents Discovered</div>
                <div style={{ fontSize: 28, fontWeight: 700, color: "var(--accent)" }}>
                  {metrics.agents_discovered}
                </div>
              </div>

              {/* Simple bar visualization */}
              {metrics.tasks_created > 0 && (
                <div style={{ ...cardStyle, marginTop: 10 }}>
                  <div style={{ fontWeight: 600, marginBottom: 8, fontSize: 12 }}>Task Distribution</div>
                  {[
                    { label: "Completed", count: metrics.tasks_completed, color: "var(--success, #22c55e)" },
                    { label: "Failed", count: metrics.tasks_failed, color: "var(--error, #ef4444)" },
                    { label: "Cancelled", count: metrics.tasks_cancelled, color: "#6b7280" },
                    {
                      label: "In Progress",
                      count: metrics.tasks_created - metrics.tasks_completed - metrics.tasks_failed - metrics.tasks_cancelled,
                      color: "var(--warning, #f59e0b)",
                    },
                  ]
                    .filter((item) => item.count > 0)
                    .map((item) => (
                      <div key={item.label} style={{ marginBottom: 6 }}>
                        <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, marginBottom: 2 }}>
                          <span>{item.label}</span>
                          <span>{item.count}</span>
                        </div>
                        <div
                          style={{
                            height: 6,
                            borderRadius: 3,
                            background: "var(--bg-tertiary)",
                            overflow: "hidden",
                          }}
                        >
                          <div
                            style={{
                              height: "100%",
                              width: `${(item.count / metrics.tasks_created) * 100}%`,
                              background: item.color,
                              borderRadius: 3,
                              transition: "width 0.3s ease",
                            }}
                          />
                        </div>
                      </div>
                    ))}
                </div>
              )}
            </>
          )}

          {!metricsLoading && !metrics && (
            <div style={{ color: "var(--text-muted)", padding: 24, textAlign: "center" }}>
              No metrics available yet.
            </div>
          )}
        </div>
      )}
    </div>
  );
}
