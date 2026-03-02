import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// ── Types ──────────────────────────────────────────────────────────────────────

interface AgentInstance {
  id: string;
  task: string;
  /** "pending" | "running" | "done" | "failed" */
  status: AgentStatus;
  step_count: number;
  branch: string;
  worktree_path: string;
}

type AgentStatus = "pending" | "running" | "done" | "failed";

interface AgentStepEvent {
  id: string;
  step_num: number;
  tool: string;
  success: boolean;
}

interface NewTaskDraft {
  task: string;
  id: string;
}

interface ManagerViewProps {
  provider: string;
}

// ── Helpers ────────────────────────────────────────────────────────────────────

function statusColor(status: AgentStatus): string {
  switch (status) {
    case "pending": return "var(--text-secondary)";
    case "running": return "#5af";
    case "done":    return "#4c4";
    case "failed":  return "#f44";
  }
}

function statusIcon(status: AgentStatus): string {
  switch (status) {
    case "pending": return "⏳";
    case "running": return "⚡";
    case "done":    return "✅";
    case "failed":  return "❌";
  }
}

let _idCounter = 0;
function nextId(): string {
  return `agent-${Date.now()}-${++_idCounter}`;
}

// ── Agent Card ─────────────────────────────────────────────────────────────────

function AgentCard({
  agent,
  steps,
  onMerge,
}: {
  agent: AgentInstance;
  steps: AgentStepEvent[];
  onMerge: (id: string, strategy: string) => void;
}) {
  const [expanded, setExpanded] = useState(false);
  const [mergeStrategy, setMergeStrategy] = useState("merge");

  return (
    <div style={{
      border: "1px solid var(--border-color)",
      borderRadius: "6px",
      marginBottom: "8px",
      background: "var(--bg-secondary)",
      overflow: "hidden",
    }}>
      {/* Header */}
      <div
        onClick={() => setExpanded(!expanded)}
        style={{
          display: "flex",
          alignItems: "center",
          padding: "10px 12px",
          cursor: "pointer",
          gap: "10px",
          userSelect: "none",
        }}
      >
        <span style={{ fontSize: "16px" }}>{statusIcon(agent.status)}</span>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{
            fontSize: "12px",
            fontWeight: 600,
            color: "var(--text-primary)",
            whiteSpace: "nowrap",
            overflow: "hidden",
            textOverflow: "ellipsis",
          }}>
            {agent.task || `Agent ${agent.id}`}
          </div>
          <div style={{ fontSize: "10px", color: "var(--text-secondary)", marginTop: "2px" }}>
            <span style={{ color: statusColor(agent.status), marginRight: "8px" }}>
              {agent.status}
            </span>
            {agent.step_count > 0 && (
              <span>{agent.step_count} step{agent.step_count !== 1 ? "s" : ""}</span>
            )}
            {agent.branch && (
              <span style={{ marginLeft: "8px", opacity: 0.6 }}>
                🌿 {agent.branch}
              </span>
            )}
          </div>
        </div>

        {/* Actions for completed agents */}
        {agent.status === "done" && (
          <div
            onClick={(e) => e.stopPropagation()}
            style={{ display: "flex", gap: "6px", alignItems: "center" }}
          >
            <select
              value={mergeStrategy}
              onChange={(e) => setMergeStrategy(e.target.value)}
              style={{
                fontSize: "10px",
                padding: "2px 4px",
                background: "var(--bg-primary)",
                border: "1px solid var(--border-color)",
                borderRadius: "3px",
                color: "var(--text-primary)",
              }}
            >
              <option value="merge">Merge</option>
              <option value="squash">Squash</option>
              <option value="rebase">Rebase</option>
            </select>
            <button
              onClick={() => onMerge(agent.id, mergeStrategy)}
              style={{
                fontSize: "10px",
                padding: "3px 8px",
                background: "var(--accent-blue, #007acc)",
                border: "none",
                borderRadius: "3px",
                color: "#fff",
                cursor: "pointer",
              }}
            >
              Merge
            </button>
          </div>
        )}

        <span style={{ fontSize: "10px", color: "var(--text-secondary)" }}>
          {expanded ? "▲" : "▼"}
        </span>
      </div>

      {/* Expanded step trace */}
      {expanded && steps.length > 0 && (
        <div style={{ borderTop: "1px solid var(--border-color)", padding: "8px 12px" }}>
          <div style={{ fontSize: "10px", fontWeight: 600, color: "var(--text-secondary)", marginBottom: "6px", textTransform: "uppercase", letterSpacing: "0.06em" }}>
            Step Trace
          </div>
          {steps.map((s, i) => (
            <div key={i} style={{ display: "flex", gap: "8px", marginBottom: "3px", alignItems: "center" }}>
              <span style={{ fontSize: "10px", color: "var(--text-secondary)", minWidth: "20px", textAlign: "right" }}>
                {s.step_num + 1}.
              </span>
              <span style={{ fontSize: "10px", color: s.success ? "#4c4" : "#f44" }}>
                {s.success ? "✔" : "✘"}
              </span>
              <span style={{ fontSize: "11px", fontFamily: "monospace", color: "var(--accent-blue, #007acc)" }}>
                {s.tool}
              </span>
            </div>
          ))}
        </div>
      )}

      {expanded && steps.length === 0 && agent.status === "running" && (
        <div style={{ borderTop: "1px solid var(--border-color)", padding: "10px 12px", fontSize: "11px", color: "var(--text-secondary)" }}>
          Waiting for first tool call…
        </div>
      )}
    </div>
  );
}

// ── ManagerView ────────────────────────────────────────────────────────────────

export function ManagerView({ provider }: ManagerViewProps) {
  const [agents, setAgents] = useState<Map<string, AgentInstance>>(new Map());
  const [steps, setSteps] = useState<Map<string, AgentStepEvent[]>>(new Map());
  const [drafts, setDrafts] = useState<NewTaskDraft[]>([{ task: "", id: nextId() }]);
  const [mergeResult, setMergeResult] = useState<string | null>(null);
  const [launching, setLaunching] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Subscribe to Tauri events
  useEffect(() => {
    const unsubs: UnlistenFn[] = [];

    listen<AgentInstance>("manager:agent_update", (e) => {
      const update = e.payload;
      setAgents((prev) => {
        const next = new Map(prev);
        const existing = next.get(update.id);
        if (existing) {
          // Merge: preserve task text on status-only updates
          next.set(update.id, {
            ...existing,
            status: update.status as AgentStatus,
            step_count: update.step_count || existing.step_count,
            // If update has an error message in task, store it
            ...(update.status === "failed" && update.task ? { task: update.task } : {}),
          });
        } else {
          next.set(update.id, { ...update, status: update.status as AgentStatus });
        }
        return next;
      });
    }).then((fn) => unsubs.push(fn)).catch(() => {});

    listen<AgentStepEvent>("manager:agent_step", (e) => {
      const step = e.payload;
      setSteps((prev) => {
        const next = new Map(prev);
        const list = next.get(step.id) ?? [];
        next.set(step.id, [...list, step]);
        return next;
      });
    }).then((fn) => unsubs.push(fn)).catch(() => {});

    return () => unsubs.forEach((fn) => fn());
  }, []);

  const handleLaunch = useCallback(async () => {
    const validTasks = drafts.filter((d) => d.task.trim().length > 0);
    if (validTasks.length === 0) return;

    setLaunching(true);
    setError(null);
    setMergeResult(null);

    try {
      const instances: AgentInstance[] = await invoke("start_parallel_agents", {
        tasks: validTasks.map((d) => ({ id: d.id, task: d.task, depends_on: [] })),
        provider,
        approvalPolicy: "full-auto",
      });

      setAgents((prev) => {
        const next = new Map(prev);
        instances.forEach((inst) => {
          next.set(inst.id, { ...inst, status: inst.status as AgentStatus });
        });
        return next;
      });

      // Reset drafts to a single blank
      setDrafts([{ task: "", id: nextId() }]);
    } catch (e: unknown) {
      setError(String(e));
    } finally {
      setLaunching(false);
    }
  }, [drafts, provider]);

  const handleMerge = useCallback(async (id: string, strategy: string) => {
    setMergeResult(null);
    try {
      const msg: string = await invoke("merge_agent_branch", { agentId: id, strategy });
      setMergeResult(msg);
    } catch (e: unknown) {
      setMergeResult(`Error: ${e}`);
    }
  }, []);

  const addDraft = () => {
    setDrafts((prev) => [...prev, { task: "", id: nextId() }]);
  };

  const updateDraft = (id: string, task: string) => {
    setDrafts((prev) => prev.map((d) => (d.id === id ? { ...d, task } : d)));
  };

  const removeDraft = (id: string) => {
    setDrafts((prev) => prev.filter((d) => d.id !== id));
  };

  const agentList = Array.from(agents.values());
  const running = agentList.filter((a) => a.status === "running").length;
  const done    = agentList.filter((a) => a.status === "done").length;
  const failed  = agentList.filter((a) => a.status === "failed").length;

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      {/* Header */}
      <div style={{
        padding: "10px 12px",
        borderBottom: "1px solid var(--border-color)",
        display: "flex",
        alignItems: "center",
        gap: "8px",
        flexShrink: 0,
      }}>
        <span style={{ fontSize: "16px" }}>🎛️</span>
        <div style={{ flex: 1 }}>
          <div style={{ fontSize: "13px", fontWeight: 600, color: "var(--text-primary)" }}>
            Manager View
          </div>
          {agentList.length > 0 && (
            <div style={{ fontSize: "10px", color: "var(--text-secondary)" }}>
              {running > 0 && <span style={{ color: "#5af", marginRight: "8px" }}>⚡ {running} running</span>}
              {done > 0 && <span style={{ color: "#4c4", marginRight: "8px" }}>✅ {done} done</span>}
              {failed > 0 && <span style={{ color: "#f44" }}>❌ {failed} failed</span>}
            </div>
          )}
        </div>
      </div>

      {/* Content */}
      <div style={{ flex: 1, overflowY: "auto", padding: "10px" }}>
        {/* Task composition area */}
        <div style={{
          background: "var(--bg-secondary)",
          border: "1px solid var(--border-color)",
          borderRadius: "6px",
          padding: "10px",
          marginBottom: "12px",
        }}>
          <div style={{ fontSize: "11px", fontWeight: 600, color: "var(--text-secondary)", marginBottom: "8px", textTransform: "uppercase", letterSpacing: "0.07em" }}>
            New Parallel Tasks
          </div>

          {drafts.map((draft, i) => (
            <div key={draft.id} style={{ display: "flex", gap: "6px", marginBottom: "6px" }}>
              <span style={{ fontSize: "10px", color: "var(--text-secondary)", padding: "6px 0", minWidth: "16px", textAlign: "right" }}>
                {i + 1}.
              </span>
              <input
                type="text"
                value={draft.task}
                onChange={(e) => updateDraft(draft.id, e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && i === drafts.length - 1) addDraft();
                }}
                placeholder={`Task ${i + 1} description…`}
                style={{
                  flex: 1,
                  padding: "5px 8px",
                  fontSize: "12px",
                  background: "var(--bg-input, var(--bg-primary))",
                  border: "1px solid var(--border-color)",
                  borderRadius: "4px",
                  color: "var(--text-primary)",
                  outline: "none",
                }}
              />
              {drafts.length > 1 && (
                <button
                  onClick={() => removeDraft(draft.id)}
                  style={{
                    padding: "4px 7px",
                    fontSize: "11px",
                    background: "none",
                    border: "1px solid var(--border-color)",
                    borderRadius: "3px",
                    color: "var(--text-secondary)",
                    cursor: "pointer",
                  }}
                >
                  ✕
                </button>
              )}
            </div>
          ))}

          <div style={{ display: "flex", gap: "6px", marginTop: "8px" }}>
            <button
              onClick={addDraft}
              style={{
                fontSize: "11px",
                padding: "4px 10px",
                background: "none",
                border: "1px dashed var(--border-color)",
                borderRadius: "3px",
                color: "var(--text-secondary)",
                cursor: "pointer",
              }}
            >
              + Add task
            </button>
            <button
              onClick={handleLaunch}
              disabled={launching || drafts.every((d) => !d.task.trim())}
              style={{
                fontSize: "11px",
                padding: "4px 14px",
                background: launching ? "var(--bg-secondary)" : "var(--accent-blue, #007acc)",
                border: "none",
                borderRadius: "3px",
                color: launching ? "var(--text-secondary)" : "#fff",
                cursor: launching ? "not-allowed" : "pointer",
                fontWeight: 600,
              }}
            >
              {launching ? "Launching…" : `Launch ${drafts.filter((d) => d.task.trim()).length} Agent${drafts.filter((d) => d.task.trim()).length !== 1 ? "s" : ""}`}
            </button>
          </div>

          {error && (
            <div style={{ marginTop: "8px", fontSize: "11px", color: "#f44" }}>
              {error}
            </div>
          )}
        </div>

        {/* Merge result toast */}
        {mergeResult && (
          <div style={{
            padding: "8px 10px",
            marginBottom: "10px",
            background: mergeResult.startsWith("Error") ? "#f441" : "#4c41",
            border: `1px solid ${mergeResult.startsWith("Error") ? "#f44" : "#4c4"}`,
            borderRadius: "4px",
            fontSize: "11px",
            color: mergeResult.startsWith("Error") ? "#f44" : "#4c4",
          }}>
            {mergeResult}
            <button
              onClick={() => setMergeResult(null)}
              style={{ marginLeft: "8px", background: "none", border: "none", cursor: "pointer", color: "inherit", fontSize: "11px" }}
            >
              ✕
            </button>
          </div>
        )}

        {/* Agent cards */}
        {agentList.length === 0 ? (
          <div style={{ padding: "24px 0", textAlign: "center", color: "var(--text-secondary)" }}>
            <div style={{ fontSize: "24px", marginBottom: "8px" }}>🎛️</div>
            <div style={{ fontSize: "13px" }}>No agents running yet.</div>
            <div style={{ fontSize: "11px", marginTop: "4px", opacity: 0.7 }}>
              Add tasks above and click Launch to start parallel agents.
            </div>
          </div>
        ) : (
          agentList.map((agent) => (
            <AgentCard
              key={agent.id}
              agent={agent}
              steps={steps.get(agent.id) ?? []}
              onMerge={handleMerge}
            />
          ))
        )}
      </div>
    </div>
  );
}
