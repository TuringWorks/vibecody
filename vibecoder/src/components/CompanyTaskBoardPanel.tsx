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


function ownerBadge(owner: TaskOwner): React.CSSProperties {
  const color = owner === 'principal' ? 'var(--accent-gold)' : owner === 'assistant' ? 'var(--accent-blue)' : 'var(--accent-green)';
  return {
    display: 'inline-block', padding: '1px 6px', borderRadius: "var(--radius-md)", fontSize: "var(--font-size-xs)", fontWeight: 600,
    background: `rgba(0,0,0,0.15)`, color, border: `1px solid ${color}`,
  };
}

function programBadge(_program: TaskProgram): React.CSSProperties {
  return {
    display: 'inline-block', padding: '1px 6px', borderRadius: "var(--radius-md)", fontSize: "var(--font-size-xs)", fontWeight: 500,
    background: 'rgba(128,128,128,0.12)', color: 'var(--text-secondary)',
    border: '1px solid var(--border-color)',
  };
}

const STATUS_LABELS: Record<string, string> = {
  backlog: "Backlog", todo: "To Do", in_progress: "In Progress",
  in_review: "In Review", done: "Done", blocked: "Blocked",
};

export function CompanyTaskBoardPanel({ workspacePath: _wp }: CompanyTaskBoardPanelProps) {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [rawOutput, setRawOutput] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [newTitle, setNewTitle] = useState("");
  const [newOwner, setNewOwner] = useState<TaskOwner>("agent");
  const [newProgram, setNewProgram] = useState<TaskProgram>("Other");
  const [newRecurrence, setNewRecurrence] = useState("");
  const [cmdResult, setCmdResult] = useState<string | null>(null);
  const [useStructured, setUseStructured] = useState(false);
  // Board drag state
  const [dragId, setDragId] = useState<string | null>(null);
  const [dragOverCol, setDragOverCol] = useState<string | null>(null);
  const [hoveredCard, setHoveredCard] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    try {
      // Try structured first, fallback to text
      const result = await invoke<Task[]>("company_task_list_json", {
        status: null,
      }).catch(async () => {
        setUseStructured(false);
        const out = await invoke<string>("company_cmd", { args: "task list" });
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

  useEffect(() => { load(); }, []);

  const moveTask = async (id: string, newStatus: string) => {
    try {
      await invoke("company_task_move", { id, status: newStatus });
      load();
    } catch (_e) {
      // fallback: just update local state optimistically
      setTasks(prev => prev.map(t => t.id === id ? { ...t, status: newStatus } : t));
    }
  };

  const onDragStart = (e: React.DragEvent, id: string) => {
    setDragId(id); e.dataTransfer.effectAllowed = "move";
  };
  const onDragOver = (e: React.DragEvent, col: string) => {
    e.preventDefault(); e.dataTransfer.dropEffect = "move"; setDragOverCol(col);
  };
  const onDragLeave = () => setDragOverCol(null);
  const onDrop = async (e: React.DragEvent, col: string) => {
    e.preventDefault(); setDragOverCol(null);
    if (!dragId) return;
    const task = tasks.find(t => t.id === dragId);
    if (task && task.status !== col) await moveTask(task.id, col);
    setDragId(null);
  };
  const onDragEnd = () => { setDragId(null); setDragOverCol(null); };

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
        <h3>Agent Tasks</h3>
        <button onClick={load} className="panel-btn panel-btn-secondary" style={{ marginLeft: "auto" }}>
          Refresh
        </button>
      </div>

      <div className="panel-body">
        {/* Create task form */}
        <div className="panel-card" style={{ marginBottom: 12 }}>
          <div style={{ fontSize: "var(--font-size-sm)", fontWeight: 700, color: "var(--text-secondary)", marginBottom: 6, textTransform: "uppercase", letterSpacing: "0.05em" }}>New Task</div>
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            <input
              value={newTitle}
              onChange={(e) => setNewTitle(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && createTask()}
              placeholder="Task title…"
              className="panel-input panel-input-full"
            />
            <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
              <select value={newOwner} onChange={(e) => setNewOwner(e.target.value as TaskOwner)} className="panel-select" style={{ flex: 1 }}>
                {OWNERS.map((o) => <option key={o} value={o}>{o}</option>)}
              </select>
              <select value={newProgram} onChange={(e) => setNewProgram(e.target.value as TaskProgram)} className="panel-select" style={{ flex: 1 }}>
                {PROGRAMS.map((p) => <option key={p} value={p}>{p}</option>)}
              </select>
              <input value={newRecurrence} onChange={(e) => setNewRecurrence(e.target.value)} placeholder="Recurrence (optional)" className="panel-input" style={{ flex: 1 }} title="e.g. daily, weekdays, weekly" />
              <button onClick={createTask} className="panel-btn panel-btn-primary">+ Task</button>
            </div>
          </div>
        </div>

        {cmdResult && <div className="panel-card" style={{ marginBottom: 10, fontSize: "var(--font-size-base)" }}>{cmdResult}</div>}

        {/* Kanban board */}
        {loading ? (
          <div className="panel-loading">Loading…</div>
        ) : useStructured ? (
          tasks.length === 0 ? (
            <div className="panel-empty">
              <div style={{ marginBottom: 8, display: "flex", justifyContent: "center", color: "var(--accent-blue)" }}><ClipboardList size={32} strokeWidth={1.5} /></div>
              <div style={{ fontWeight: 600, marginBottom: 4 }}>No tasks yet</div>
              <div style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-base)" }}>Use the form above to create your first task</div>
            </div>
          ) : (
            <div style={{ display: "flex", gap: 12, overflowX: "auto", paddingBottom: 8 }}>
              {STATUSES.map(status => {
                const colTasks = tasks.filter(t => t.status === status);
                const isDragTarget = dragOverCol === status;
                return (
                  <div
                    key={status}
                    onDragOver={e => onDragOver(e, status)}
                    onDragLeave={onDragLeave}
                    onDrop={e => onDrop(e, status)}
                    style={{
                      minWidth: 200, flex: 1, borderRadius: "var(--radius-md)", padding: 10,
                      transition: "background 0.15s, border 0.15s",
                      background: isDragTarget ? "color-mix(in srgb, var(--accent-blue) 8%, transparent)" : "var(--bg-secondary)",
                      border: isDragTarget ? "2px dashed var(--accent-blue)" : "1px solid var(--border-color)",
                    }}
                  >
                    {/* Column header */}
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
                      <span style={{ fontWeight: 600, fontSize: "var(--font-size-md)", color: "var(--text-primary)" }}>{STATUS_LABELS[status] ?? status.replace("_", " ")}</span>
                      <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>{colTasks.length}</span>
                    </div>

                    {/* Cards */}
                    {colTasks.map(task => {
                      const isDragging = dragId === task.id;
                      return (
                        <div
                          key={task.id}
                          draggable
                          onDragStart={e => onDragStart(e, task.id)}
                          onDragEnd={onDragEnd}
                          onMouseEnter={() => setHoveredCard(task.id)}
                          onMouseLeave={() => setHoveredCard(null)}
                          style={{
                            background: "var(--bg-elevated)", border: "1px solid var(--border-color)",
                            borderRadius: "var(--radius-md)", padding: 10, marginBottom: 8,
                            cursor: "grab", opacity: isDragging ? 0.4 : 1,
                            transition: "var(--transition-fast)",
                            transform: hoveredCard === task.id && !isDragging ? "translateY(-2px)" : "none",
                            boxShadow: hoveredCard === task.id ? "var(--elevation-2)" : "var(--card-shadow)",
                          }}
                        >
                          <div style={{ fontWeight: 500, fontSize: "var(--font-size-md)", color: "var(--text-primary)", marginBottom: 6 }}>{task.title}</div>
                          <div style={{ display: "flex", gap: 4, flexWrap: "wrap", alignItems: "center" }}>
                            <span style={ownerBadge(task.owner)}>{task.owner}</span>
                            <span style={programBadge(task.program)}>{task.program}</span>
                            {task.recurrence && (
                              <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", padding: "1px 4px", background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)" }}>
                                {task.recurrence}
                              </span>
                            )}
                          </div>
                          {/* ← → move buttons */}
                          <div style={{ display: "flex", gap: 4, marginTop: 8 }}>
                            {STATUSES.indexOf(status) > 0 && (
                              <button className="panel-btn panel-btn-secondary" style={{ padding: "2px 8px", fontSize: "var(--font-size-sm)" }} onClick={() => moveTask(task.id, STATUSES[STATUSES.indexOf(status) - 1])}>&larr;</button>
                            )}
                            {STATUSES.indexOf(status) < STATUSES.length - 1 && (
                              <button className="panel-btn panel-btn-secondary" style={{ padding: "2px 8px", fontSize: "var(--font-size-sm)" }} onClick={() => moveTask(task.id, STATUSES[STATUSES.indexOf(status) + 1])}>&rarr;</button>
                            )}
                          </div>
                        </div>
                      );
                    })}

                    {colTasks.length === 0 && (
                      <div style={{ textAlign: "center", padding: "20px 8px", color: "var(--text-muted)", fontSize: "var(--font-size-sm)", opacity: 0.5 }}>Drop here</div>
                    )}
                  </div>
                );
              })}
            </div>
          )
        ) : (
          !rawOutput || rawOutput.includes("No tasks") ? (
            <div className="panel-empty">
              <div style={{ marginBottom: 8, display: "flex", justifyContent: "center", color: "var(--accent-blue)" }}><ClipboardList size={32} strokeWidth={1.5} /></div>
              <div style={{ fontWeight: 600, marginBottom: 4 }}>No tasks yet</div>
              <div style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-base)" }}>Use the form above to create your first task</div>
            </div>
          ) : (
            <div className="panel-card" style={{ minHeight: 120 }}>
              <pre style={{ margin: 0, fontSize: "var(--font-size-base)", whiteSpace: "pre-wrap", lineHeight: 1.6 }}>{rawOutput}</pre>
            </div>
          )
        )}
      </div>
    </div>
  );
}
