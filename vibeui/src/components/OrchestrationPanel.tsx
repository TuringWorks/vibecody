import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

/* ── Types ───────────────────────────────────────────────────────────── */

interface TodoItem {
  id: number;
  description: string;
  done: boolean;
  stepType: string;
}

interface Lesson {
  id: number;
  pattern: string;
  rule: string;
  category: string;
  hitCount: number;
}

interface OrchestrationState {
  goal: string;
  complexity: "trivial" | "moderate" | "complex";
  todos: TodoItem[];
  planned: boolean;
  verified: boolean;
}

/* ── Helpers ─────────────────────────────────────────────────────────── */

const COMPLEXITY_COLORS: Record<string, string> = {
  trivial: "var(--success-color)",
  moderate: "var(--warning-color)",
  complex: "var(--error-color)",
};

const STEP_ICONS: Record<string, string> = {
  build: "B",
  plan: "P",
  research: "R",
  verify: "V",
  test: "T",
  review: "W",
};

/* ── Component ───────────────────────────────────────────────────────── */

interface OrchestrationPanelProps {
  workspacePath: string | null;
}

export function OrchestrationPanel({ workspacePath: _workspacePath }: OrchestrationPanelProps) {
  const [activeTab, setActiveTab] = useState<"tasks" | "lessons" | "rules">("tasks");

  const [state, setState] = useState<OrchestrationState>({
    goal: "",
    complexity: "moderate",
    todos: [],
    planned: false,
    verified: false,
  });

  const [lessons, setLessons] = useState<Lesson[]>([]);

  // Load persisted state on mount
  useEffect(() => {
    invoke<OrchestrationState>("get_orch_state").then(s => setState(s)).catch(() => {});
    invoke<Lesson[]>("get_orch_lessons").then(l => setLessons(l)).catch(() => {});
  }, []);

  // Forms
  const [newGoal, setNewGoal] = useState("");
  const [newTodo, setNewTodo] = useState("");
  const [newPattern, setNewPattern] = useState("");
  const [newRule, setNewRule] = useState("");
  const [newCategory, setNewCategory] = useState("general");

  /* ── Task actions ─────────────────────────────────────────────────── */

  const persistState = useCallback((s: OrchestrationState) => {
    setState(s);
    invoke("save_orch_state", { state: s }).catch(() => {});
  }, []);

  const persistLessons = useCallback((l: Lesson[]) => {
    setLessons(l);
    invoke("save_orch_lessons", { lessons: l }).catch(() => {});
  }, []);

  const createTask = useCallback(() => {
    if (!newGoal.trim()) return;
    persistState({
      goal: newGoal,
      complexity: newGoal.length > 80 ? "complex" : "moderate",
      todos: [],
      planned: false,
      verified: false,
    });
    setNewGoal("");
  }, [newGoal, persistState]);

  const addTodo = useCallback(() => {
    if (!newTodo.trim()) return;
    const nextId = state.todos.length > 0 ? Math.max(...state.todos.map(t => t.id)) + 1 : 1;
    const updated = { ...state, todos: [...state.todos, { id: nextId, description: newTodo, done: false, stepType: "build" }] };
    persistState(updated);
    setNewTodo("");
  }, [newTodo, state, persistState]);

  const toggleTodo = useCallback((id: number) => {
    const updated = { ...state, todos: state.todos.map(t => t.id === id ? { ...t, done: !t.done } : t) };
    persistState(updated);
  }, [state, persistState]);

  const markVerified = useCallback(() => {
    persistState({ ...state, verified: true });
  }, [state, persistState]);

  const markPlanned = useCallback(() => {
    persistState({ ...state, planned: true });
  }, [state, persistState]);

  const resetTask = useCallback(() => {
    persistState({ goal: "", complexity: "moderate", todos: [], planned: false, verified: false });
  }, [persistState]);

  /* ── Lesson actions ───────────────────────────────────────────────── */

  const addLesson = useCallback(() => {
    if (!newPattern.trim()) return;
    const nextId = lessons.length > 0 ? Math.max(...lessons.map(l => l.id)) + 1 : 1;
    const updated = [...lessons, { id: nextId, pattern: newPattern, rule: newRule, category: newCategory, hitCount: 0 }];
    persistLessons(updated);
    setNewPattern("");
    setNewRule("");
  }, [newPattern, newRule, newCategory, lessons, persistLessons]);

  const deleteLesson = useCallback((id: number) => {
    persistLessons(lessons.filter(l => l.id !== id));
  }, [lessons, persistLessons]);

  /* ── Progress calculation ─────────────────────────────────────────── */

  const completed = state.todos.filter(t => t.done).length;
  const total = state.todos.length;
  const pct = total > 0 ? Math.round((completed / total) * 100) : 0;
  const allDone = total > 0 && completed === total;
  const readyToClose = allDone && (state.complexity !== "complex" || state.verified);

  /* ── Render ───────────────────────────────────────────────────────── */

  const tabStyle = (active: boolean): React.CSSProperties => ({
    padding: "6px 16px",
    cursor: "pointer",
    borderBottom: active ? "2px solid var(--accent-blue)" : "2px solid transparent",
    color: active ? "var(--text-primary)" : "var(--text-secondary)",
    background: "none",
    border: "none",
    borderBottomWidth: "2px",
    borderBottomStyle: "solid",
    borderBottomColor: active ? "var(--accent-blue)" : "transparent",
    fontSize: "13px",
    fontWeight: active ? 600 : 400,
  });

  return (
    <div style={{ padding: "12px", fontFamily: "var(--font-family)", color: "var(--text-primary)", height: "100%", overflow: "auto" }}>
      {/* Tabs */}
      <div style={{ display: "flex", gap: 2, borderBottom: "1px solid var(--border-color)", padding: "0 16px", flexShrink: 0 }}>
        <button style={tabStyle(activeTab === "tasks")} onClick={() => setActiveTab("tasks")}>Tasks</button>
        <button style={tabStyle(activeTab === "lessons")} onClick={() => setActiveTab("lessons")}>Lessons</button>
        <button style={tabStyle(activeTab === "rules")} onClick={() => setActiveTab("rules")}>Rules</button>
      </div>

      {/* Tasks tab */}
      {activeTab === "tasks" && (
        <div>
          {!state.goal ? (
            <div>
              <h3 style={{ margin: "0 0 8px", fontSize: "14px" }}>New Task</h3>
              <div style={{ display: "flex", gap: "8px" }}>
                <input
                  value={newGoal}
                  onChange={e => setNewGoal(e.target.value)}
                  onKeyDown={e => e.key === "Enter" && createTask()}
                  placeholder="Describe the task goal..."
                  style={{ flex: 1, padding: "6px 8px", background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "4px", fontSize: "13px" }}
                />
                <button onClick={createTask} style={{ padding: "6px 12px", background: "var(--accent-color)", color: "var(--btn-primary-fg)", border: "none", borderRadius: "4px", cursor: "pointer", fontSize: "13px" }}>Create</button>
              </div>
            </div>
          ) : (
            <div>
              {/* Status header */}
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "12px" }}>
                <div>
                  <h3 style={{ margin: 0, fontSize: "14px" }}>{state.goal}</h3>
                  <div style={{ display: "flex", gap: "12px", fontSize: "12px", color: "var(--text-secondary)", marginTop: "4px" }}>
                    <span style={{ color: COMPLEXITY_COLORS[state.complexity] }}>{state.complexity}</span>
                    <span>{state.planned ? "Planned" : "Not planned"}</span>
                    <span>{state.verified ? "Verified" : "Not verified"}</span>
                  </div>
                </div>
                <div style={{ display: "flex", gap: "4px" }}>
                  {!state.planned && <button onClick={markPlanned} style={{ padding: "4px 8px", background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "3px", cursor: "pointer", fontSize: "11px" }}>Mark Planned</button>}
                  {!state.verified && <button onClick={markVerified} style={{ padding: "4px 8px", background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "3px", cursor: "pointer", fontSize: "11px" }}>Verify</button>}
                  <button onClick={resetTask} style={{ padding: "4px 8px", background: "var(--error-color)", color: "var(--btn-primary-fg)", border: "none", borderRadius: "3px", cursor: "pointer", fontSize: "11px" }}>Reset</button>
                </div>
              </div>

              {/* Progress bar */}
              <div style={{ background: "var(--bg-primary)", borderRadius: "4px", height: "6px", marginBottom: "12px" }}>
                <div style={{ background: readyToClose ? "var(--success-color)" : "var(--accent-color)", width: `${pct}%`, height: "100%", borderRadius: "4px", transition: "width 0.2s" }} />
              </div>
              <div style={{ fontSize: "11px", color: "var(--text-secondary)", marginBottom: "12px" }}>
                {completed}/{total} tasks ({pct}%)
                {readyToClose && <span style={{ color: "var(--success-color)", marginLeft: "8px" }}>Ready to close</span>}
              </div>

              {/* Todo list */}
              {state.todos.map(todo => (
                <div key={todo.id} role="checkbox" aria-checked={todo.done} tabIndex={0} onClick={() => toggleTodo(todo.id)} onKeyDown={e => e.key === "Enter" && toggleTodo(todo.id)} style={{ display: "flex", alignItems: "center", gap: "8px", padding: "4px 0", cursor: "pointer", opacity: todo.done ? 0.5 : 1 }}>
                  <span style={{ width: "16px", height: "16px", border: "1px solid var(--border-color)", borderRadius: "3px", display: "flex", alignItems: "center", justifyContent: "center", fontSize: "10px", background: todo.done ? "var(--success-color)" : "transparent", color: "var(--btn-primary-fg)" }}>
                    {todo.done ? "x" : ""}
                  </span>
                  <span style={{ fontSize: "10px", padding: "1px 4px", background: "var(--bg-primary)", borderRadius: "2px", color: "var(--text-secondary)" }}>
                    {STEP_ICONS[todo.stepType] || "B"}
                  </span>
                  <span style={{ fontSize: "13px", textDecoration: todo.done ? "line-through" : "none" }}>{todo.description}</span>
                </div>
              ))}

              {/* Add todo */}
              <div style={{ display: "flex", gap: "8px", marginTop: "12px" }}>
                <input
                  value={newTodo}
                  onChange={e => setNewTodo(e.target.value)}
                  onKeyDown={e => e.key === "Enter" && addTodo()}
                  placeholder="Add a task item..."
                  style={{ flex: 1, padding: "6px 8px", background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "4px", fontSize: "13px" }}
                />
                <button onClick={addTodo} style={{ padding: "6px 12px", background: "var(--accent-color)", color: "var(--btn-primary-fg)", border: "none", borderRadius: "4px", cursor: "pointer", fontSize: "13px" }}>Add</button>
              </div>
            </div>
          )}
        </div>
      )}

      {/* Lessons tab */}
      {activeTab === "lessons" && (
        <div>
          <h3 style={{ margin: "0 0 8px", fontSize: "14px" }}>Lessons Learned ({lessons.length})</h3>

          {lessons.length === 0 && (
            <p style={{ color: "var(--text-secondary)", fontSize: "13px" }}>No lessons yet. Record patterns to prevent repeated mistakes.</p>
          )}

          {lessons.map(lesson => (
            <div key={lesson.id} style={{ padding: "8px", marginBottom: "6px", background: "var(--bg-primary)", borderRadius: "4px", border: "1px solid var(--border-color)" }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <span style={{ fontSize: "11px", padding: "1px 6px", background: "var(--border-color)", borderRadius: "2px", color: "var(--text-secondary)" }}>{lesson.category}</span>
                <button onClick={() => deleteLesson(lesson.id)} style={{ background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer", fontSize: "11px" }}>x</button>
              </div>
              <div style={{ fontSize: "13px", marginTop: "4px" }}>
                <span style={{ color: "var(--warning-color)" }}>{lesson.pattern}</span>
                {lesson.rule && <span style={{ color: "var(--text-secondary)" }}> &rarr; </span>}
                {lesson.rule && <span style={{ color: "var(--success-color)" }}>{lesson.rule}</span>}
              </div>
            </div>
          ))}

          {/* Add lesson form */}
          <div style={{ marginTop: "12px", display: "flex", flexDirection: "column", gap: "6px" }}>
            <div style={{ display: "flex", gap: "8px" }}>
              <select value={newCategory} onChange={e => setNewCategory(e.target.value)} style={{ padding: "6px", background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "4px", fontSize: "12px" }}>
                <option value="general">general</option>
                <option value="rust">rust</option>
                <option value="typescript">typescript</option>
                <option value="testing">testing</option>
                <option value="security">security</option>
                <option value="performance">performance</option>
                <option value="architecture">architecture</option>
              </select>
              <input value={newPattern} onChange={e => setNewPattern(e.target.value)} placeholder="Pattern / mistake..." style={{ flex: 1, padding: "6px 8px", background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "4px", fontSize: "13px" }} />
            </div>
            <div style={{ display: "flex", gap: "8px" }}>
              <input
                value={newRule}
                onChange={e => setNewRule(e.target.value)}
                onKeyDown={e => e.key === "Enter" && addLesson()}
                placeholder="Rule to prevent recurrence..."
                style={{ flex: 1, padding: "6px 8px", background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "4px", fontSize: "13px" }}
              />
              <button onClick={addLesson} style={{ padding: "6px 12px", background: "var(--accent-color)", color: "var(--btn-primary-fg)", border: "none", borderRadius: "4px", cursor: "pointer", fontSize: "13px" }}>Add</button>
            </div>
          </div>
        </div>
      )}

      {/* Rules tab */}
      {activeTab === "rules" && (
        <div style={{ fontSize: "13px", lineHeight: 1.6 }}>
          <h3 style={{ margin: "0 0 12px", fontSize: "14px" }}>Orchestration Rules</h3>

          <div style={{ padding: "8px", background: "var(--bg-primary)", borderRadius: "4px", border: "1px solid var(--border-color)", marginBottom: "8px" }}>
            <strong style={{ color: "var(--accent-color)" }}>1. Plan Node Default</strong>
            <p style={{ margin: "4px 0 0", color: "var(--text-secondary)" }}>Enter plan mode for non-trivial tasks (3+ steps). Stop and re-plan if things go sideways.</p>
          </div>

          <div style={{ padding: "8px", background: "var(--bg-primary)", borderRadius: "4px", border: "1px solid var(--border-color)", marginBottom: "8px" }}>
            <strong style={{ color: "var(--accent-color)" }}>2. Subagent Strategy</strong>
            <p style={{ margin: "4px 0 0", color: "var(--text-secondary)" }}>Offload research and exploration to subagents. One task per subagent.</p>
          </div>

          <div style={{ padding: "8px", background: "var(--bg-primary)", borderRadius: "4px", border: "1px solid var(--border-color)", marginBottom: "8px" }}>
            <strong style={{ color: "var(--accent-color)" }}>3. Self-Improvement Loop</strong>
            <p style={{ margin: "4px 0 0", color: "var(--text-secondary)" }}>After any correction: record the lesson. Review lessons at session start.</p>
          </div>

          <div style={{ padding: "8px", background: "var(--bg-primary)", borderRadius: "4px", border: "1px solid var(--border-color)", marginBottom: "8px" }}>
            <strong style={{ color: "var(--accent-color)" }}>4. Verification Before Done</strong>
            <p style={{ margin: "4px 0 0", color: "var(--text-secondary)" }}>Never close without proving it works. Run tests, check logs, demonstrate correctness.</p>
          </div>

          <div style={{ padding: "8px", background: "var(--bg-primary)", borderRadius: "4px", border: "1px solid var(--border-color)", marginBottom: "8px" }}>
            <strong style={{ color: "var(--accent-color)" }}>5. Demand Elegance</strong>
            <p style={{ margin: "4px 0 0", color: "var(--text-secondary)" }}>For non-trivial changes, pause and ask "is there a more elegant way?" Skip for simple fixes.</p>
          </div>

          <div style={{ padding: "8px", background: "var(--bg-primary)", borderRadius: "4px", border: "1px solid var(--border-color)", marginBottom: "8px" }}>
            <strong style={{ color: "var(--accent-color)" }}>6. Autonomous Bug Fixing</strong>
            <p style={{ margin: "4px 0 0", color: "var(--text-secondary)" }}>Read logs, fix bugs, zero hand-holding. Go fix failing CI without being told how.</p>
          </div>

          <div style={{ padding: "8px", background: "var(--bg-primary)", borderRadius: "4px", border: "1px solid var(--border-color)", marginBottom: "8px" }}>
            <strong style={{ color: "var(--accent-color)" }}>Core Principles</strong>
            <ul style={{ margin: "4px 0 0", paddingLeft: "20px", color: "var(--text-secondary)" }}>
              <li>Simplicity First — minimal code impact</li>
              <li>No Laziness — find root causes, no temporary fixes</li>
              <li>Minimal Impact — only touch what's necessary</li>
            </ul>
          </div>
        </div>
      )}
    </div>
  );
}

export default OrchestrationPanel;
