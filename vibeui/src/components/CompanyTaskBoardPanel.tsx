/**
 * CompanyTaskBoardPanel — Kanban task board with status columns.
 *
 * Shows tasks grouped by status: Backlog, Todo, In Progress, In Review,
 * Done, Blocked. Supports creating tasks with owner, program, recurrence.
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ClipboardList } from "lucide-react";

const STATUSES = ["backlog", "todo", "in_progress", "in_review", "done", "blocked"] as const;
const OWNERS = ["principal", "assistant", "agent"] as const;
const PROGRAMS = ["Revenue", "EA", "Legal", "BizDev", "Marketing", "Product", "Personal", "Other"] as const;

type TaskOwner = typeof OWNERS[number];
type TaskProgram = typeof PROGRAMS[number];

interface Task {
  id: string;
  title: string;
  status: string;
  owner: TaskOwner;
  program: TaskProgram;
  recurrence: string | null;
  created_at: number;
}

interface CompanyTaskBoardPanelProps {
  workspacePath?: string | null;
}

const inputStyle: React.CSSProperties = {
  fontSize: 12, padding: "4px 8px",
  background: "var(--bg-primary)",
  border: "1px solid var(--border-color)", borderRadius: 4,
  color: "var(--text-primary)",
};

function ownerBadge(owner: TaskOwner): React.CSSProperties {
  const color = owner === 'principal' ? 'var(--accent-gold)' : owner === 'assistant' ? 'var(--accent-blue)' : 'var(--accent-green)';
  return {
    display: 'inline-block', padding: '1px 6px', borderRadius: 10, fontSize: 10, fontWeight: 600,
    background: `rgba(0,0,0,0.15)`, color, border: `1px solid ${color}`,
  };
}

function programBadge(_program: TaskProgram): React.CSSProperties {
  return {
    display: 'inline-block', padding: '1px 6px', borderRadius: 10, fontSize: 10, fontWeight: 500,
    background: 'rgba(128,128,128,0.12)', color: 'var(--text-secondary)',
    border: '1px solid var(--border-color)',
  };
}

export function CompanyTaskBoardPanel({ workspacePath: _wp }: CompanyTaskBoardPanelProps) {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [rawOutput, setRawOutput] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [newTitle, setNewTitle] = useState("");
  const [newOwner, setNewOwner] = useState<TaskOwner>("agent");
  const [newProgram, setNewProgram] = useState<TaskProgram>("Other");
  const [newRecurrence, setNewRecurrence] = useState("");
  const [cmdResult, setCmdResult] = useState<string | null>(null);
  const [filterStatus, setFilterStatus] = useState<string>("");
  const [useStructured, setUseStructured] = useState(false);

  const load = async () => {
    setLoading(true);
    try {
      // Try structured first, fallback to text
      const result = await invoke<Task[]>("company_task_list_json", {
        status: filterStatus || null,
      }).catch(async () => {
        setUseStructured(false);
        const args = filterStatus ? `task list --status ${filterStatus}` : "task list";
        const out = await invoke<string>("company_cmd", { args });
        setRawOutput(out);
        return null;
      });
      if (result !== null) {
        setUseStructured(true);
        setTasks(result);
      }
    } catch (e) {
      setRawOutput(`Error: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, [filterStatus]);

  const createTask = async () => {
    if (!newTitle.trim()) return;
    try {
      await invoke("company_task_create_v2", {
        title: newTitle.trim(),
        status: "backlog",
        owner: newOwner,
        program: newProgram,
        recurrence: newRecurrence.trim() || null,
      });
      setCmdResult("Task created.");
      setNewTitle("");
      setNewRecurrence("");
      load();
    } catch (_e) {
      // fallback to legacy
      try {
        const out = await invoke<string>("company_cmd", { args: `task create ${newTitle.trim()}` });
        setCmdResult(out);
        setNewTitle("");
        load();
      } catch (e2) {
        setCmdResult(`Error: ${e2}`);
      }
    }
  };

  return (
    <div className="panel-container">
      <div className="panel-header">
        <span style={{ fontWeight: 600, fontSize: 14 }}>Agent Tasks</span>
        <button onClick={load} className="panel-btn panel-btn-secondary">
          Refresh
        </button>
      </div>

      <div className="panel-body">
        {/* Filter by status */}
        <div style={{ display: "flex", gap: 6, marginBottom: 12, flexWrap: "wrap" }}>
          <button
            onClick={() => setFilterStatus("")}
            style={{
              fontSize: 11, padding: "2px 8px", cursor: "pointer",
              background: filterStatus === "" ? "var(--accent-blue)" : "var(--bg-tertiary)",
              color: filterStatus === "" ? "#fff" : "var(--text-primary)",
              border: `1px solid ${filterStatus === "" ? "var(--accent-blue)" : "var(--border-color)"}`,
              borderRadius: 12,
            }}
          >
            All
          </button>
          {STATUSES.map((s) => (
            <button
              key={s}
              onClick={() => setFilterStatus(s)}
              style={{
                fontSize: 11, padding: "2px 8px", cursor: "pointer", borderRadius: 12,
                background: filterStatus === s ? "var(--accent-blue)" : "var(--bg-tertiary)",
                color: filterStatus === s ? "#fff" : "var(--text-primary)",
                border: `1px solid ${filterStatus === s ? "var(--accent-blue)" : "var(--border-color)"}`,
              }}
            >
              {s.replace("_", " ")}
            </button>
          ))}
        </div>

        {/* Create task form */}
        <div className="panel-card" style={{ marginBottom: 16 }}>
          <div style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)", marginBottom: 8 }}>NEW TASK</div>
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            <input
              value={newTitle}
              onChange={(e) => setNewTitle(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && createTask()}
              placeholder="Task title…"
              style={{ ...inputStyle, width: "100%", boxSizing: "border-box" }}
            />
            <div style={{ display: "flex", gap: 6 }}>
              <select
                value={newOwner}
                onChange={(e) => setNewOwner(e.target.value as TaskOwner)}
                style={{ ...inputStyle, flex: 1 }}
              >
                {OWNERS.map((o) => <option key={o} value={o}>{o}</option>)}
              </select>
              <select
                value={newProgram}
                onChange={(e) => setNewProgram(e.target.value as TaskProgram)}
                style={{ ...inputStyle, flex: 1 }}
              >
                {PROGRAMS.map((p) => <option key={p} value={p}>{p}</option>)}
              </select>
              <input
                value={newRecurrence}
                onChange={(e) => setNewRecurrence(e.target.value)}
                placeholder="Recurrence (optional)"
                style={{ ...inputStyle, flex: 1 }}
                title="e.g. daily, weekdays, weekly, or cron expression"
              />
              <button onClick={createTask} className="panel-btn panel-btn-primary">
                + Task
              </button>
            </div>
          </div>
        </div>

        {cmdResult && (
          <div className="panel-card" style={{ marginBottom: 12, fontSize: 12 }}>
            {cmdResult}
          </div>
        )}

        {/* Task display */}
        {useStructured ? (
          tasks.length === 0 && !loading ? (
            <div className="panel-empty">
              <div style={{ marginBottom: 8, display: "flex", justifyContent: "center", color: "var(--accent-blue)" }}><ClipboardList size={32} strokeWidth={1.5} /></div>
              <div style={{ fontWeight: 600, marginBottom: 4 }}>No tasks yet</div>
              <div style={{ color: "var(--text-secondary)", fontSize: 12 }}>Use the form above to create your first task</div>
            </div>
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
              {loading ? (
                <div className="panel-loading">Loading…</div>
              ) : (
                tasks.map((task) => (
                  <div key={task.id} className="panel-card" style={{ padding: "8px 12px" }}>
                    <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                      <span style={{ fontSize: 12, flex: 1 }}>{task.title}</span>
                      <span style={ownerBadge(task.owner)}>{task.owner}</span>
                      <span style={programBadge(task.program)}>{task.program}</span>
                      {task.recurrence && (
                        <span style={{ fontSize: 10, color: "var(--text-secondary)", padding: "1px 5px", background: "rgba(0,0,0,0.1)", borderRadius: 6 }}>
                          {task.recurrence}
                        </span>
                      )}
                      <span style={{ fontSize: 10, color: "var(--text-secondary)", minWidth: 70 }}>{task.status.replace("_", " ")}</span>
                    </div>
                  </div>
                ))
              )}
            </div>
          )
        ) : (
          !loading && (!rawOutput || rawOutput.includes("No tasks")) ? (
            <div className="panel-empty">
              <div style={{ marginBottom: 8, display: "flex", justifyContent: "center", color: "var(--accent-blue)" }}><ClipboardList size={32} strokeWidth={1.5} /></div>
              <div style={{ fontWeight: 600, marginBottom: 4 }}>No tasks yet</div>
              <div style={{ color: "var(--text-secondary)", fontSize: 12, marginBottom: 4 }}>
                Use the form above to create your first task
              </div>
              <div style={{ color: "var(--text-secondary)", fontSize: 11 }}>
                Workflow: backlog → todo → in_progress → in_review → done
              </div>
            </div>
          ) : (
            <div className="panel-card" style={{ minHeight: 120 }}>
              {loading ? (
                <div className="panel-loading">Loading…</div>
              ) : (
                <pre style={{ margin: 0, fontSize: 12, whiteSpace: "pre-wrap", lineHeight: 1.6 }}>
                  {rawOutput}
                </pre>
              )}
            </div>
          )
        )}
      </div>
    </div>
  );
}
