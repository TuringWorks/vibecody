/**
 * SpawnAgentPanel — Parallel agent spawning & lifecycle management UI
 *
 * Sub-tabs:
 * - Active: Live view of running/queued/paused agents with progress
 * - Spawn: Create new agents or decompose tasks into subtasks
 * - Results: Aggregated results from completed agents
 * - History: Past agents with outcomes and metrics
 */
import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

/* ── Types ─────────────────────────────────── */
type SubTab = "active" | "spawn" | "results" | "history";
type SpawnStatus = "queued" | "running" | "paused" | "completed" | "failed" | "cancelled";
type AgentPriority = "low" | "normal" | "high" | "critical";
type IsolationMode = "worktree" | "container" | "none";
type DecomposeStrategy = "by_file" | "by_concern" | "by_component" | "custom";
type MergeStrategy = "best_result" | "sequential_merge" | "cherry_pick" | "manual";
type MessageType = "status" | "request" | "response" | "file_change" | "conflict" | "done";

interface AgentProgress {
  turns_completed: number;
  turns_limit: number;
  files_modified: string[];
  last_message: string | null;
  tool_calls: number;
  tokens_used: number;
  percent_complete: number;
}

interface AgentMessage {
  from_id: string;
  to_id: string;
  msg_type: MessageType;
  content: string;
  timestamp: number;
}

interface SpawnedAgent {
  id: string;
  name: string;
  task: string;
  status: SpawnStatus;
  config: {
    task: string;
    name: string | null;
    provider: string | null;
    model: string | null;
    max_turns: number;
    isolation: IsolationMode;
    priority: AgentPriority;
    context_files: string[];
    background: boolean;
    approval_policy: string;
    parent_id: string | null;
    tags: string[];
    timeout_secs: number;
  };
  progress: AgentProgress;
  branch: string | null;
  worktree_path: string | null;
  result_summary: string | null;
  error: string | null;
  created_at: number;
  started_at: number | null;
  finished_at: number | null;
  parent_id: string | null;
  child_ids: string[];
  inbox: AgentMessage[];
}

interface PoolStats {
  total: number;
  queued: number;
  running: number;
  paused: number;
  completed: number;
  failed: number;
  cancelled: number;
  max_concurrent: number;
  max_total: number;
  total_tokens: number;
  total_files_modified: number;
}

interface AgentSummary {
  agent_id: string;
  agent_name: string;
  status: SpawnStatus;
  summary: string | null;
  files_modified: number;
  turns_taken: number;
  tokens_used: number;
  duration_ms: number;
  branch: string | null;
}

interface MergeConflict {
  file: string;
  agent_a: string;
  agent_b: string;
  description: string;
}

interface AggregatedResult {
  strategy: MergeStrategy;
  total_agents: number;
  successful_agents: number;
  failed_agents: number;
  best_agent_id: string | null;
  merged_branch: string | null;
  summaries: AgentSummary[];
  conflicts: MergeConflict[];
  total_files_modified: number;
  total_tokens_used: number;
  total_duration_ms: number;
}

/* ── Helpers ───────────────────────────────── */
const statusColor = (s: SpawnStatus) =>
  s === "running" ? "var(--vscode-testing-runAction)" :
  s === "completed" ? "var(--vscode-testing-iconPassed)" :
  s === "failed" ? "var(--vscode-testing-iconFailed)" :
  s === "paused" ? "var(--vscode-charts-yellow)" :
  s === "queued" ? "var(--vscode-descriptionForeground)" :
  "var(--vscode-descriptionForeground)";

const statusIcon = (s: SpawnStatus) =>
  s === "running" ? "\u25B6" :
  s === "completed" ? "\u2714" :
  s === "failed" ? "\u2718" :
  s === "paused" ? "\u23F8" :
  s === "queued" ? "\u23F3" :
  "\u2717";

const formatDuration = (ms: number) => {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${Math.floor(ms / 60000)}m${Math.floor((ms % 60000) / 1000)}s`;
};

const formatTime = (ts: number) => new Date(ts).toLocaleTimeString();

/* ── Component ─────────────────────────────── */
export default function SpawnAgentPanel() {
  const [tab, setTab] = useState<SubTab>("active");
  const [agents, setAgents] = useState<SpawnedAgent[]>([]);
  const [stats, setStats] = useState<PoolStats | null>(null);
  const [selectedAgent, setSelectedAgent] = useState<string | null>(null);

  // Spawn form
  const [task, setTask] = useState("");
  const [agentName, setAgentName] = useState("");
  const [priority, setPriority] = useState<AgentPriority>("normal");
  const [isolation, setIsolation] = useState<IsolationMode>("worktree");
  const [maxTurns, setMaxTurns] = useState(25);
  const [contextFiles, setContextFiles] = useState("");
  const [decompose, setDecompose] = useState(false);
  const [decomposeStrategy, setDecomposeStrategy] = useState<DecomposeStrategy>("by_concern");

  // Results
  const [aggregatedResult, setAggregatedResult] = useState<AggregatedResult | null>(null);

  /* ── Data loading ─────────────────────────── */
  const refreshData = useCallback(async () => {
    try {
      const [agentList, poolStats] = await Promise.all([
        invoke<SpawnedAgent[]>("spawn_agent_list"),
        invoke<PoolStats>("spawn_agent_stats"),
      ]);
      setAgents(agentList);
      setStats(poolStats);
    } catch {
      // Backend not available yet
    }
  }, []);

  useEffect(() => {
    refreshData();
    const interval = setInterval(refreshData, 2000);
    return () => clearInterval(interval);
  }, [refreshData]);

  /* ── Actions ──────────────────────────────── */
  const handleSpawn = async () => {
    if (!task.trim()) return;
    try {
      if (decompose) {
        await invoke("spawn_agent_decompose", {
          task: task.trim(),
          strategy: decomposeStrategy,
          contextFiles: contextFiles.split("\n").filter(Boolean),
        });
      } else {
        await invoke("spawn_agent_new", {
          config: {
            task: task.trim(),
            name: agentName || null,
            priority,
            isolation,
            max_turns: maxTurns,
            context_files: contextFiles.split("\n").filter(Boolean),
            background: true,
            approval_policy: "full-auto",
            tags: [],
            timeout_secs: 0,
          },
        });
      }
      setTask("");
      setAgentName("");
      refreshData();
    } catch (e) {
      console.error("Spawn failed:", e);
    }
  };

  const handleAction = async (action: string, agentId: string) => {
    try {
      await invoke(`spawn_agent_${action}`, { agentId });
      refreshData();
    } catch (e) {
      console.error(`Action ${action} failed:`, e);
    }
  };

  const handleAggregate = async (parentId: string) => {
    try {
      const result = await invoke<AggregatedResult>("spawn_agent_aggregate", { parentId });
      setAggregatedResult(result);
      setTab("results");
    } catch (e) {
      console.error("Aggregate failed:", e);
    }
  };

  /* ── Render helpers ───────────────────────── */
  const activeAgents = agents.filter(a => ["running", "queued", "paused"].includes(a.status));
  const historyAgents = agents.filter(a => ["completed", "failed", "cancelled"].includes(a.status));
  const selected = agents.find(a => a.id === selectedAgent);

  const tabStyle = (t: SubTab) => ({
    padding: "6px 14px",
    cursor: "pointer" as const,
    borderBottom: tab === t ? "2px solid var(--vscode-focusBorder)" : "2px solid transparent",
    color: tab === t ? "var(--vscode-foreground)" : "var(--vscode-descriptionForeground)",
    background: "none",
    border: "none",
    borderBottomStyle: "solid" as const,
    borderBottomWidth: 2,
    borderBottomColor: tab === t ? "var(--vscode-focusBorder)" : "transparent",
    fontSize: 13,
  });

  const cardStyle = {
    border: "1px solid var(--vscode-panel-border)",
    borderRadius: 4,
    padding: 10,
    marginBottom: 8,
    background: "var(--vscode-editor-background)",
  };

  const inputStyle = {
    width: "100%",
    padding: "6px 8px",
    background: "var(--vscode-input-background)",
    color: "var(--vscode-input-foreground)",
    border: "1px solid var(--vscode-input-border)",
    borderRadius: 3,
    fontSize: 13,
    boxSizing: "border-box" as const,
  };

  const btnStyle = {
    padding: "6px 14px",
    cursor: "pointer" as const,
    background: "var(--vscode-button-background)",
    color: "var(--vscode-button-foreground)",
    border: "none",
    borderRadius: 3,
    fontSize: 13,
  };

  const smallBtnStyle = {
    ...btnStyle,
    padding: "3px 8px",
    fontSize: 12,
    background: "var(--vscode-button-secondaryBackground)",
    color: "var(--vscode-button-secondaryForeground)",
  };

  /* ── Progress bar ─────────────────────────── */
  const ProgressBar = ({ percent, status }: { percent: number; status: SpawnStatus }) => (
    <div style={{ height: 4, background: "var(--vscode-input-border)", borderRadius: 2, marginTop: 4 }}>
      <div style={{
        height: "100%",
        width: `${Math.min(percent, 100)}%`,
        background: statusColor(status),
        borderRadius: 2,
        transition: "width 0.3s ease",
      }} />
    </div>
  );

  /* ── Active Tab ───────────────────────────── */
  const renderActive = () => (
    <div>
      {stats && (
        <div style={{ display: "flex", gap: 12, marginBottom: 12, flexWrap: "wrap", fontSize: 12, color: "var(--vscode-descriptionForeground)" }}>
          <span>{stats.running} running</span>
          <span>{stats.queued} queued</span>
          <span>{stats.paused} paused</span>
          <span>{stats.completed} done</span>
          <span>{stats.failed} failed</span>
          <span>|</span>
          <span>{stats.total}/{stats.max_total} total</span>
          <span>{stats.total_tokens.toLocaleString()} tokens</span>
        </div>
      )}

      {activeAgents.length === 0 ? (
        <div style={{ color: "var(--vscode-descriptionForeground)", padding: 20, textAlign: "center" }}>
          No active agents. Use the Spawn tab to create one.
        </div>
      ) : (
        activeAgents.map(a => (
          <div key={a.id} style={{
            ...cardStyle,
            borderLeft: `3px solid ${statusColor(a.status)}`,
            cursor: "pointer",
          }} onClick={() => setSelectedAgent(a.id)}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <span style={{ color: statusColor(a.status), marginRight: 6 }}>{statusIcon(a.status)}</span>
                <strong>{a.name}</strong>
                <span style={{ color: "var(--vscode-descriptionForeground)", marginLeft: 8, fontSize: 11 }}>{a.id}</span>
              </div>
              <div style={{ display: "flex", gap: 4 }}>
                {a.status === "running" && (
                  <button style={smallBtnStyle} onClick={e => { e.stopPropagation(); handleAction("pause", a.id); }}>Pause</button>
                )}
                {a.status === "paused" && (
                  <button style={smallBtnStyle} onClick={e => { e.stopPropagation(); handleAction("resume", a.id); }}>Resume</button>
                )}
                {!["completed", "failed", "cancelled"].includes(a.status) && (
                  <button style={{ ...smallBtnStyle, color: "var(--vscode-testing-iconFailed)" }}
                    onClick={e => { e.stopPropagation(); handleAction("cancel", a.id); }}>
                    Stop
                  </button>
                )}
                {a.child_ids.length > 0 && (
                  <button style={smallBtnStyle} onClick={e => { e.stopPropagation(); handleAggregate(a.id); }}>
                    Results
                  </button>
                )}
              </div>
            </div>

            <div style={{ fontSize: 12, color: "var(--vscode-descriptionForeground)", marginTop: 4 }}>{a.task}</div>

            <ProgressBar percent={a.progress.percent_complete} status={a.status} />

            <div style={{ display: "flex", gap: 12, marginTop: 6, fontSize: 11, color: "var(--vscode-descriptionForeground)" }}>
              <span>{a.progress.turns_completed}/{a.progress.turns_limit} turns</span>
              <span>{a.progress.files_modified.length} files</span>
              <span>{a.progress.tokens_used.toLocaleString()} tokens</span>
              {a.branch && <span>branch: {a.branch}</span>}
              <span>{a.config.priority} priority</span>
            </div>

            {a.progress.last_message && (
              <div style={{ fontSize: 11, marginTop: 4, fontStyle: "italic", color: "var(--vscode-descriptionForeground)" }}>
                {a.progress.last_message}
              </div>
            )}
          </div>
        ))
      )}

      {/* Selected agent detail */}
      {selected && (
        <div style={{ ...cardStyle, marginTop: 12, borderTop: "2px solid var(--vscode-focusBorder)" }}>
          <h4 style={{ margin: "0 0 8px" }}>{selected.name} — Detail</h4>
          <div style={{ fontSize: 12, lineHeight: 1.6 }}>
            <div><strong>ID:</strong> {selected.id}</div>
            <div><strong>Task:</strong> {selected.task}</div>
            <div><strong>Status:</strong> {selected.status} | <strong>Priority:</strong> {selected.config.priority}</div>
            <div><strong>Isolation:</strong> {selected.config.isolation}</div>
            {selected.branch && <div><strong>Branch:</strong> {selected.branch}</div>}
            <div><strong>Created:</strong> {formatTime(selected.created_at)}</div>
            {selected.started_at && <div><strong>Started:</strong> {formatTime(selected.started_at)}</div>}
            {selected.progress.files_modified.length > 0 && (
              <div>
                <strong>Files modified:</strong>
                <ul style={{ margin: "2px 0", paddingLeft: 16 }}>
                  {selected.progress.files_modified.map(f => <li key={f}>{f}</li>)}
                </ul>
              </div>
            )}
            {selected.inbox.length > 0 && (
              <div>
                <strong>Messages ({selected.inbox.length}):</strong>
                {selected.inbox.slice(-5).map((m, i) => (
                  <div key={i} style={{ fontSize: 11, marginLeft: 8, color: "var(--vscode-descriptionForeground)" }}>
                    [{m.msg_type}] {m.from_id}: {m.content}
                  </div>
                ))}
              </div>
            )}
            {selected.child_ids.length > 0 && (
              <div><strong>Subtasks:</strong> {selected.child_ids.join(", ")}</div>
            )}
          </div>
        </div>
      )}
    </div>
  );

  /* ── Spawn Tab ────────────────────────────── */
  const renderSpawn = () => (
    <div>
      <div style={{ marginBottom: 12 }}>
        <label style={{ fontSize: 12, display: "block", marginBottom: 4 }}>Task Description</label>
        <textarea
          value={task}
          onChange={e => setTask(e.target.value)}
          placeholder="Describe the task for the agent..."
          rows={3}
          style={{ ...inputStyle, resize: "vertical" }}
        />
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10, marginBottom: 12 }}>
        <div>
          <label style={{ fontSize: 12, display: "block", marginBottom: 4 }}>Agent Name (optional)</label>
          <input value={agentName} onChange={e => setAgentName(e.target.value)} style={inputStyle} placeholder="auto-generated" />
        </div>
        <div>
          <label style={{ fontSize: 12, display: "block", marginBottom: 4 }}>Priority</label>
          <select value={priority} onChange={e => setPriority(e.target.value as AgentPriority)} style={inputStyle}>
            <option value="low">Low</option>
            <option value="normal">Normal</option>
            <option value="high">High</option>
            <option value="critical">Critical</option>
          </select>
        </div>
        <div>
          <label style={{ fontSize: 12, display: "block", marginBottom: 4 }}>Isolation</label>
          <select value={isolation} onChange={e => setIsolation(e.target.value as IsolationMode)} style={inputStyle}>
            <option value="worktree">Git Worktree</option>
            <option value="container">Docker Container</option>
            <option value="none">None (shared workspace)</option>
          </select>
        </div>
        <div>
          <label style={{ fontSize: 12, display: "block", marginBottom: 4 }}>Max Turns</label>
          <input type="number" value={maxTurns} onChange={e => setMaxTurns(Number(e.target.value))} style={inputStyle} min={1} max={200} />
        </div>
      </div>

      <div style={{ marginBottom: 12 }}>
        <label style={{ fontSize: 12, display: "block", marginBottom: 4 }}>Context Files (one per line)</label>
        <textarea
          value={contextFiles}
          onChange={e => setContextFiles(e.target.value)}
          placeholder="src/main.rs&#10;src/lib.rs"
          rows={2}
          style={{ ...inputStyle, resize: "vertical" }}
        />
      </div>

      <div style={{ marginBottom: 12, display: "flex", alignItems: "center", gap: 10 }}>
        <label style={{ fontSize: 12, display: "flex", alignItems: "center", gap: 4 }}>
          <input type="checkbox" checked={decompose} onChange={e => setDecompose(e.target.checked)} />
          Decompose into parallel subtasks
        </label>
        {decompose && (
          <select value={decomposeStrategy} onChange={e => setDecomposeStrategy(e.target.value as DecomposeStrategy)} style={{ ...inputStyle, width: "auto" }}>
            <option value="by_concern">By Concern (implement + test + docs)</option>
            <option value="by_file">By File</option>
            <option value="by_component">By Component (directory)</option>
          </select>
        )}
      </div>

      <button style={btnStyle} onClick={handleSpawn} disabled={!task.trim()}>
        {decompose ? "Decompose & Spawn" : "Spawn Agent"}
      </button>
    </div>
  );

  /* ── Results Tab ──────────────────────────── */
  const renderResults = () => (
    <div>
      {!aggregatedResult ? (
        <div style={{ color: "var(--vscode-descriptionForeground)", padding: 20, textAlign: "center" }}>
          Select a coordinator agent and click "Results" to aggregate subtask outputs.
        </div>
      ) : (
        <div>
          <div style={{ display: "flex", gap: 16, marginBottom: 12, flexWrap: "wrap", fontSize: 13 }}>
            <span><strong>Strategy:</strong> {aggregatedResult.strategy.replace("_", " ")}</span>
            <span><strong>Agents:</strong> {aggregatedResult.successful_agents}/{aggregatedResult.total_agents} successful</span>
            <span><strong>Files:</strong> {aggregatedResult.total_files_modified}</span>
            <span><strong>Tokens:</strong> {aggregatedResult.total_tokens_used.toLocaleString()}</span>
            <span><strong>Duration:</strong> {formatDuration(aggregatedResult.total_duration_ms)}</span>
          </div>

          {aggregatedResult.best_agent_id && (
            <div style={{ ...cardStyle, borderLeft: "3px solid var(--vscode-testing-iconPassed)" }}>
              Best agent: <strong>{aggregatedResult.best_agent_id}</strong>
            </div>
          )}

          {aggregatedResult.conflicts.length > 0 && (
            <div style={{ marginBottom: 12 }}>
              <h4 style={{ color: "var(--vscode-charts-yellow)", margin: "0 0 6px" }}>
                Conflicts ({aggregatedResult.conflicts.length})
              </h4>
              {aggregatedResult.conflicts.map((c, i) => (
                <div key={i} style={{ ...cardStyle, borderLeft: "3px solid var(--vscode-charts-yellow)", fontSize: 12 }}>
                  <strong>{c.file}</strong> - {c.description}
                </div>
              ))}
            </div>
          )}

          <h4 style={{ margin: "0 0 6px" }}>Agent Summaries</h4>
          {aggregatedResult.summaries.map(s => (
            <div key={s.agent_id} style={{
              ...cardStyle,
              borderLeft: `3px solid ${statusColor(s.status)}`,
            }}>
              <div style={{ display: "flex", justifyContent: "space-between" }}>
                <div>
                  <span style={{ color: statusColor(s.status), marginRight: 4 }}>{statusIcon(s.status)}</span>
                  <strong>{s.agent_name}</strong>
                  <span style={{ marginLeft: 8, fontSize: 11, color: "var(--vscode-descriptionForeground)" }}>{s.agent_id}</span>
                </div>
                {s.branch && <span style={{ fontSize: 11 }}>{s.branch}</span>}
              </div>
              <div style={{ fontSize: 12, marginTop: 4, display: "flex", gap: 12, color: "var(--vscode-descriptionForeground)" }}>
                <span>{s.files_modified} files</span>
                <span>{s.turns_taken} turns</span>
                <span>{s.tokens_used.toLocaleString()} tokens</span>
                <span>{formatDuration(s.duration_ms)}</span>
              </div>
              {s.summary && <div style={{ fontSize: 12, marginTop: 4 }}>{s.summary}</div>}
            </div>
          ))}
        </div>
      )}
    </div>
  );

  /* ── History Tab ──────────────────────────── */
  const renderHistory = () => (
    <div>
      {historyAgents.length === 0 ? (
        <div style={{ color: "var(--vscode-descriptionForeground)", padding: 20, textAlign: "center" }}>
          No completed agents yet.
        </div>
      ) : (
        historyAgents.map(a => (
          <div key={a.id} style={{
            ...cardStyle,
            borderLeft: `3px solid ${statusColor(a.status)}`,
            opacity: a.status === "cancelled" ? 0.6 : 1,
          }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <span style={{ color: statusColor(a.status), marginRight: 6 }}>{statusIcon(a.status)}</span>
                <strong>{a.name}</strong>
                <span style={{ fontSize: 11, color: "var(--vscode-descriptionForeground)", marginLeft: 8 }}>{a.id}</span>
              </div>
              <span style={{ fontSize: 11, color: "var(--vscode-descriptionForeground)" }}>
                {a.finished_at ? formatTime(a.finished_at) : ""}
              </span>
            </div>
            <div style={{ fontSize: 12, color: "var(--vscode-descriptionForeground)", marginTop: 4 }}>{a.task}</div>
            <div style={{ display: "flex", gap: 12, marginTop: 4, fontSize: 11, color: "var(--vscode-descriptionForeground)" }}>
              <span>{a.progress.turns_completed} turns</span>
              <span>{a.progress.files_modified.length} files</span>
              <span>{a.progress.tokens_used.toLocaleString()} tokens</span>
              {a.branch && <span>branch: {a.branch}</span>}
            </div>
            {a.result_summary && (
              <div style={{ fontSize: 12, marginTop: 4, color: "var(--vscode-testing-iconPassed)" }}>
                {a.result_summary}
              </div>
            )}
            {a.error && (
              <div style={{ fontSize: 12, marginTop: 4, color: "var(--vscode-testing-iconFailed)" }}>
                {a.error}
              </div>
            )}
          </div>
        ))
      )}
    </div>
  );

  /* ── Main render ──────────────────────────── */
  return (
    <div style={{ padding: 12 }}>
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--vscode-panel-border)", marginBottom: 12 }}>
        {(["active", "spawn", "results", "history"] as SubTab[]).map(t => (
          <button key={t} style={tabStyle(t)} onClick={() => setTab(t)}>
            {t === "active" ? `Active (${activeAgents.length})` :
             t === "history" ? `History (${historyAgents.length})` :
             t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      {tab === "active" && renderActive()}
      {tab === "spawn" && renderSpawn()}
      {tab === "results" && renderResults()}
      {tab === "history" && renderHistory()}
    </div>
  );
}
