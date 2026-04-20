import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Circle,
  Flame,
  ListTodo,
  Loader2,
  Plus,
  RefreshCw,
  Sliders,
  Terminal,
  Zap,
} from "lucide-react";
import type { TodoistTask } from "../../types/productivity";
import { ProviderStatusStrip } from "./ProviderStatusStrip";
import { TaskComposer } from "./TaskComposer";

type View = "all" | "today";

function priorityIcon(p: number) {
  if (p === 4) return <Flame size={12} color="var(--color-error, #d63e3e)" />;
  if (p === 3) return <Zap size={12} color="var(--color-warn, #c69023)" />;
  if (p === 2) return <Circle size={10} color="var(--text-secondary)" strokeWidth={2} />;
  return <span style={{ color: "var(--text-secondary)", width: 12, textAlign: "center" }}>·</span>;
}

export function TasksTab() {
  const [view, setView] = useState<View>("all");
  const [tasks, setTasks] = useState<TodoistTask[]>([]);
  const [loading, setLoading] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [newTask, setNewTask] = useState("");
  const [adding, setAdding] = useState(false);
  const [completing, setCompleting] = useState<string | null>(null);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [cmd, setCmd] = useState("");
  const [cmdOutput, setCmdOutput] = useState("");
  const [cmdBusy, setCmdBusy] = useState(false);
  const [composing, setComposing] = useState(false);

  const fetchTasks = useCallback(async (v: View) => {
    setLoading(true);
    setErr(null);
    try {
      const filter = v === "today" ? "today" : "";
      const list = await invoke<TodoistTask[]>("productivity_tasks_list", { filter });
      setTasks(list);
    } catch (e) {
      setErr(String(e));
      setTasks([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchTasks(view);
  }, [view, fetchTasks]);

  async function addTask() {
    const c = newTask.trim();
    if (!c) return;
    setAdding(true);
    try {
      const t = await invoke<TodoistTask>("productivity_tasks_add", { content: c });
      setTasks((prev) => [t, ...prev]);
      setNewTask("");
    } catch (e) {
      setErr(String(e));
    } finally {
      setAdding(false);
    }
  }

  async function closeTask(id: string) {
    setCompleting(id);
    try {
      await invoke("productivity_tasks_close", { id });
      setTasks((prev) => prev.filter((t) => t.id !== id));
    } catch (e) {
      setErr(String(e));
    } finally {
      setCompleting(null);
    }
  }

  async function runAdvancedCmd() {
    if (!cmd.trim()) return;
    setCmdBusy(true);
    try {
      const out = await invoke<string>("handle_productivity_command", {
        args: `todo ${cmd}`,
      });
      setCmdOutput(out);
    } catch (e) {
      setCmdOutput(`Error: ${e}`);
    } finally {
      setCmdBusy(false);
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", flex: 1, overflow: "hidden" }}>
      <ProviderStatusStrip tab="tasks" />
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 6,
          padding: "8px 10px",
          borderBottom: "1px solid var(--border-color)",
          flexWrap: "wrap",
        }}
      >
        <button
          className={`panel-btn panel-btn-secondary${view === "all" ? " active" : ""}`}
          onClick={() => setView("all")}
          style={{ display: "flex", alignItems: "center", gap: 4 }}
        >
          <ListTodo size={12} />
          All
        </button>
        <button
          className={`panel-btn panel-btn-secondary${view === "today" ? " active" : ""}`}
          onClick={() => setView("today")}
          style={{ display: "flex", alignItems: "center", gap: 4 }}
        >
          Today
        </button>
        <button
          className="panel-btn panel-btn-secondary"
          onClick={() => fetchTasks(view)}
          disabled={loading}
          title="Refresh"
          style={{ display: "flex", alignItems: "center", gap: 4 }}
        >
          {loading ? (
            <Loader2 size={12} style={{ animation: "spin 1s linear infinite" }} />
          ) : (
            <RefreshCw size={12} />
          )}
        </button>
        <span style={{ flex: 1 }} />
        <button
          className="panel-btn panel-btn-secondary"
          onClick={() => setShowAdvanced((s) => !s)}
          title="Advanced: raw /todo commands"
          style={{ display: "flex", alignItems: "center", gap: 4 }}
        >
          <Terminal size={12} />
        </button>
      </div>
      <div
        style={{
          display: "flex",
          gap: 6,
          padding: "8px 10px",
          borderBottom: "1px solid var(--border-color)",
        }}
      >
        <Plus size={13} color="var(--text-secondary)" style={{ alignSelf: "center" }} />
        <input
          className="panel-input"
          style={{ flex: 1 }}
          placeholder="Quick add task (supports Todoist syntax: tomorrow p1, etc.)"
          value={newTask}
          onChange={(e) => setNewTask(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") addTask();
          }}
          disabled={adding}
        />
        <button
          className="panel-btn panel-btn-primary"
          onClick={addTask}
          disabled={adding || !newTask.trim()}
        >
          {adding ? "Adding…" : "Add"}
        </button>
        <button
          className="panel-btn panel-btn-secondary"
          onClick={() => setComposing(true)}
          title="Add task with due date + priority"
          style={{ display: "flex", alignItems: "center" }}
        >
          <Sliders size={12} />
        </button>
      </div>
      {err && (
        <div
          style={{
            padding: "6px 10px",
            color: "var(--color-error, #d63e3e)",
            background: "var(--bg-secondary)",
            fontSize: "var(--font-size-sm)",
            borderBottom: "1px solid var(--border-color)",
          }}
        >
          {err}
        </div>
      )}
      <div style={{ flex: 1, overflowY: "auto" }}>
        {loading && tasks.length === 0 ? (
          <div
            style={{
              padding: 20,
              display: "flex",
              alignItems: "center",
              gap: 6,
              color: "var(--text-secondary)",
              fontSize: "var(--font-size-sm)",
            }}
          >
            <Loader2 size={13} style={{ animation: "spin 1s linear infinite" }} />
            Loading tasks…
          </div>
        ) : tasks.length === 0 ? (
          <div
            style={{
              padding: 20,
              color: "var(--text-secondary)",
              textAlign: "center",
              fontSize: "var(--font-size-sm)",
            }}
          >
            {view === "today" ? "Nothing due today." : "No active tasks."}
          </div>
        ) : (
          tasks.map((t) => (
            <div
              key={t.id}
              style={{
                display: "grid",
                gridTemplateColumns: "22px 16px 1fr auto",
                alignItems: "center",
                gap: 8,
                padding: "8px 10px",
                borderBottom: "1px solid var(--border-color)",
                fontSize: "var(--font-size-sm)",
              }}
            >
              <button
                onClick={() => closeTask(t.id)}
                disabled={completing === t.id}
                title="Complete task"
                style={{
                  background: "none",
                  border: "none",
                  padding: 0,
                  cursor: "pointer",
                  color: "inherit",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                }}
              >
                {completing === t.id ? (
                  <Loader2 size={13} style={{ animation: "spin 1s linear infinite" }} />
                ) : (
                  <Circle size={13} strokeWidth={1.5} color="var(--text-secondary)" />
                )}
              </button>
              <span style={{ display: "flex", alignItems: "center", justifyContent: "center" }}>
                {priorityIcon(t.priority)}
              </span>
              <span style={{ display: "flex", flexDirection: "column", gap: 2, overflow: "hidden" }}>
                <span style={{ whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                  {t.content}
                </span>
                {t.description && (
                  <span
                    style={{
                      color: "var(--text-secondary)",
                      fontSize: "calc(var(--font-size-sm) - 1px)",
                      whiteSpace: "nowrap",
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                    }}
                  >
                    {t.description}
                  </span>
                )}
              </span>
              <span
                style={{
                  color: "var(--text-secondary)",
                  fontSize: "calc(var(--font-size-sm) - 1px)",
                  whiteSpace: "nowrap",
                }}
              >
                {t.due ?? ""}
              </span>
            </div>
          ))
        )}
      </div>
      {composing && (
        <TaskComposer
          onClose={() => setComposing(false)}
          onCreated={(t) => setTasks((prev) => [t, ...prev])}
        />
      )}
      {showAdvanced && (
        <div
          style={{
            borderTop: "1px solid var(--border-color)",
            padding: 10,
            background: "var(--bg-secondary)",
            display: "flex",
            flexDirection: "column",
            gap: 6,
            maxHeight: "35%",
          }}
        >
          <div style={{ display: "flex", gap: 6 }}>
            <input
              className="panel-input"
              style={{ flex: 1 }}
              placeholder="todo today | todo list | todo add <task> | todo close <id>"
              value={cmd}
              onChange={(e) => setCmd(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") runAdvancedCmd();
              }}
              disabled={cmdBusy}
            />
            <button
              className="panel-btn panel-btn-primary"
              onClick={runAdvancedCmd}
              disabled={cmdBusy || !cmd.trim()}
            >
              {cmdBusy ? "Running…" : "Run"}
            </button>
          </div>
          {cmdOutput && (
            <pre
              style={{
                margin: 0,
                padding: 8,
                background: "var(--bg-primary)",
                border: "1px solid var(--border-color)",
                borderRadius: "var(--radius-xs-plus)",
                fontSize: "var(--font-size-sm)",
                whiteSpace: "pre-wrap",
                overflowY: "auto",
                flex: 1,
              }}
            >
              {cmdOutput}
            </pre>
          )}
        </div>
      )}
    </div>
  );
}
