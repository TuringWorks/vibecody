/**
 * AgilePanel — Comprehensive Agile Project Management panel.
 *
 * Tabs: Board | Sprint | Backlog | Ceremonies | Metrics | Methodology | AI Coach
 *
 * Supports Scrum, Kanban, XP, Lean, FDD, Crystal, SAFe methodologies.
 * All persistence via Tauri invoke calls.
 */
import React, { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

/* ── Types ───────────────────────────────────────────────────────────── */

type TabKey = "board" | "sprint" | "backlog" | "ceremonies" | "metrics" | "methodology" | "coach";
type Priority = "P0" | "P1" | "P2" | "P3";
type Column = "Backlog" | "To Do" | "In Progress" | "In Review" | "Done";
type SprintStatus = "Planning" | "Active" | "Completed" | "Cancelled";
type Methodology = "Scrum" | "Kanban" | "XP" | "Lean" | "FDD" | "Crystal" | "SAFe";
type RiskLevel = "red" | "amber" | "green";

const COLUMNS: Column[] = ["Backlog", "To Do", "In Progress", "In Review", "Done"];
const PRIORITIES: Priority[] = ["P0", "P1", "P2", "P3"];

interface Card {
  id: string;
  title: string;
  description: string;
  assignee: string;
  priority: Priority;
  storyPoints: number;
  labels: string[];
  column: Column;
  acceptanceCriteria: string[];
  createdAt: string;
}

interface WipLimits {
  [key: string]: number;
}

interface BoardData {
  cards: Card[];
  wipLimits: WipLimits;
}

interface Sprint {
  id: string;
  name: string;
  goal: string;
  startDate: string;
  endDate: string;
  status: SprintStatus;
  velocity: number;
  plannedPoints: number;
  completedPoints: number;
  cards: Card[];
}

interface SprintHistory {
  id: string;
  name: string;
  velocity: number;
  completedPoints: number;
  plannedPoints: number;
  status: SprintStatus;
}

interface StandupEntry {
  member: string;
  didYesterday: string;
  willDoToday: string;
  blockers: string;
}

interface RetroCard {
  id: string;
  text: string;
  category: "well" | "didnt" | "action";
}

interface CeremonyData {
  standups: StandupEntry[];
  capacity: { members: number; days: number; focusFactor: number };
  demoChecklist: { item: string; done: boolean }[];
  retro: RetroCard[];
}

interface MetricsData {
  velocityHistory: { sprint: string; points: number }[];
  cumulativeFlow: { date: string; backlog: number; todo: number; inProgress: number; inReview: number; done: number }[];
  cycleTimeDays: number;
  leadTimeDays: number;
  scopeCreepPct: number;
  plannedVsCompleted: number;
  capacityUtilization: number;
}

interface AiRecommendation {
  category: string;
  title: string;
  description: string;
  risk: RiskLevel;
  actionItems: string[];
}

interface AiAnalysis {
  taskId: string;
  recommendations: AiRecommendation[];
  bottlenecks: string[];
  sizingSuggestions: string[];
  retroInsights: string[];
}

/* ── Priority colors ────────────────────────────────────────────────── */

const PRIORITY_COLORS: Record<Priority, string> = {
  P0: "#ef4444",
  P1: "#f59e0b",
  P2: "#3b82f6",
  P3: "#6b7280",
};

const riskColor = (r: RiskLevel) =>
  r === "red" ? "var(--error-color)" : r === "amber" ? "var(--warning-color)" : "var(--success-color)";

/* ── Shared styles ──────────────────────────────────────────────────── */

const tabBarStyle: React.CSSProperties = {
  display: "flex",
  gap: 2,
  borderBottom: "1px solid var(--border-color)",
  marginBottom: 16,
  padding: "0 4px",
  overflowX: "auto",
};

const tabStyle = (active: boolean): React.CSSProperties => ({
  padding: "8px 14px",
  cursor: "pointer",
  fontSize: 13,
  fontWeight: active ? 600 : 400,
  color: active ? "var(--accent-blue)" : "var(--text-secondary)",
  borderBottom: active ? "2px solid var(--accent-blue)" : "2px solid transparent",
  background: "transparent",
  border: "none",
  borderBottomStyle: "solid",
  borderBottomWidth: 2,
  borderBottomColor: active ? "var(--accent-blue)" : "transparent",
  transition: "var(--transition-fast)",
  whiteSpace: "nowrap",
});

const cardBaseStyle: React.CSSProperties = {
  background: "var(--bg-elevated)",
  border: "1px solid var(--border-color)",
  borderRadius: "var(--radius-md)",
  padding: 12,
  marginBottom: 8,
  cursor: "pointer",
  transition: "var(--transition-fast)",
  boxShadow: "var(--card-shadow)",
};

const badgeStyle = (bg: string, fg = "#fff"): React.CSSProperties => ({
  display: "inline-block",
  padding: "2px 8px",
  borderRadius: "var(--radius-sm)",
  fontSize: 11,
  fontWeight: 600,
  background: bg,
  color: fg,
  marginRight: 4,
});

const btnStyle: React.CSSProperties = {
  padding: "6px 14px",
  borderRadius: "var(--radius-sm)",
  border: "1px solid var(--border-color)",
  background: "var(--bg-elevated)",
  color: "var(--text-primary)",
  cursor: "pointer",
  fontSize: 12,
  fontWeight: 500,
  transition: "var(--transition-fast)",
};

const btnPrimaryStyle: React.CSSProperties = {
  ...btnStyle,
  background: "var(--accent-blue)",
  color: "#fff",
  borderColor: "var(--accent-blue)",
};

const inputStyle: React.CSSProperties = {
  padding: "6px 10px",
  borderRadius: "var(--radius-sm)",
  border: "1px solid var(--border-color)",
  background: "var(--bg-secondary)",
  color: "var(--text-primary)",
  fontSize: 13,
  width: "100%",
  boxSizing: "border-box",
};

const sectionTitle: React.CSSProperties = {
  fontSize: 15,
  fontWeight: 600,
  color: "var(--text-primary)",
  marginBottom: 10,
};

/* ── Helpers ─────────────────────────────────────────────────────────── */

const genId = () => Math.random().toString(36).slice(2, 10);

const defaultCard = (column: Column): Card => ({
  id: genId(),
  title: "",
  description: "",
  assignee: "",
  priority: "P2",
  storyPoints: 0,
  labels: [],
  column,
  acceptanceCriteria: [],
  createdAt: new Date().toISOString(),
});

/* ═══════════════════════════════════════════════════════════════════════
   Board Tab (Kanban)
   ═══════════════════════════════════════════════════════════════════════ */

function BoardTab() {
  const [cards, setCards] = useState<Card[]>([]);
  const [wipLimits, setWipLimits] = useState<WipLimits>({ "Backlog": 20, "To Do": 10, "In Progress": 5, "In Review": 5, "Done": 50 });
  const [editingCard, setEditingCard] = useState<Card | null>(null);
  const [addingTo, setAddingTo] = useState<Column | null>(null);
  const [newTitle, setNewTitle] = useState("");
  const [error, setError] = useState("");
  const [hoveredCard, setHoveredCard] = useState<string | null>(null);

  useEffect(() => {
    (async () => {
      try {
        const data = await invoke<BoardData>("agile_get_board");
        setCards(data.cards || []);
        if (data.wipLimits) setWipLimits(data.wipLimits);
      } catch (e: any) {
        setError(typeof e === "string" ? e : e?.message || "Failed to load board");
      }
    })();
  }, []);

  const moveCard = useCallback(async (cardId: string, targetCol: Column) => {
    try {
      await invoke("agile_move_card", { cardId, column: targetCol });
      setCards(prev => prev.map(c => c.id === cardId ? { ...c, column: targetCol } : c));
    } catch (e: any) {
      setError(typeof e === "string" ? e : e?.message || "Failed to move card");
    }
  }, []);

  const saveCard = useCallback(async (card: Card) => {
    try {
      await invoke("agile_update_card", { card });
      setCards(prev => {
        const idx = prev.findIndex(c => c.id === card.id);
        if (idx >= 0) {
          const next = [...prev];
          next[idx] = card;
          return next;
        }
        return [...prev, card];
      });
      setEditingCard(null);
    } catch (e: any) {
      setError(typeof e === "string" ? e : e?.message || "Failed to save card");
    }
  }, []);

  const addCard = useCallback(async (col: Column) => {
    if (!newTitle.trim()) return;
    const card = { ...defaultCard(col), title: newTitle.trim() };
    await saveCard(card);
    setNewTitle("");
    setAddingTo(null);
  }, [newTitle, saveCard]);

  const colIdx = (col: Column) => COLUMNS.indexOf(col);

  return (
    <div>
      {error && <div style={{ color: "var(--error-color)", marginBottom: 8, fontSize: 12 }}>{error}</div>}
      <div style={{ display: "flex", gap: 12, overflowX: "auto", paddingBottom: 8 }}>
        {COLUMNS.map(col => {
          const colCards = cards.filter(c => c.column === col);
          const overWip = colCards.length > (wipLimits[col] || 50);
          return (
            <div key={col} style={{ minWidth: 220, flex: 1, background: "var(--bg-secondary)", borderRadius: "var(--radius-md)", padding: 10, border: overWip ? "2px solid var(--warning-color)" : "1px solid var(--border-color)" }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
                <span style={{ fontWeight: 600, fontSize: 13, color: "var(--text-primary)" }}>{col}</span>
                <span style={{ fontSize: 11, color: overWip ? "var(--warning-color)" : "var(--text-secondary)" }}>
                  {colCards.length}/{wipLimits[col] || "~"}
                </span>
              </div>
              {overWip && <div style={{ fontSize: 11, color: "var(--warning-color)", marginBottom: 6 }}>WIP limit exceeded!</div>}

              {colCards.map(card => (
                <div
                  key={card.id}
                  style={{
                    ...cardBaseStyle,
                    transform: hoveredCard === card.id ? "translateY(-1px)" : "none",
                    boxShadow: hoveredCard === card.id ? "var(--elevation-2)" : "var(--card-shadow)",
                  }}
                  onMouseEnter={() => setHoveredCard(card.id)}
                  onMouseLeave={() => setHoveredCard(null)}
                  onClick={() => setEditingCard({ ...card })}
                >
                  <div style={{ fontWeight: 500, fontSize: 13, color: "var(--text-primary)", marginBottom: 6 }}>{card.title}</div>
                  <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginBottom: 6 }}>
                    <span style={badgeStyle(PRIORITY_COLORS[card.priority])}>{card.priority}</span>
                    {card.storyPoints > 0 && <span style={badgeStyle("var(--accent-purple)")}>{card.storyPoints} pts</span>}
                    {card.labels.map(l => <span key={l} style={badgeStyle("var(--bg-tertiary)", "var(--text-secondary)")}>{l}</span>)}
                  </div>
                  {card.assignee && <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>{card.assignee}</div>}
                  {/* Move buttons */}
                  <div style={{ display: "flex", gap: 4, marginTop: 6 }} onClick={e => e.stopPropagation()}>
                    {colIdx(col) > 0 && (
                      <button style={{ ...btnStyle, padding: "2px 8px", fontSize: 11 }} onClick={() => moveCard(card.id, COLUMNS[colIdx(col) - 1])}>
                        &larr;
                      </button>
                    )}
                    {colIdx(col) < COLUMNS.length - 1 && (
                      <button style={{ ...btnStyle, padding: "2px 8px", fontSize: 11 }} onClick={() => moveCard(card.id, COLUMNS[colIdx(col) + 1])}>
                        &rarr;
                      </button>
                    )}
                  </div>
                </div>
              ))}

              {addingTo === col ? (
                <div style={{ marginTop: 6 }}>
                  <input
                    style={inputStyle}
                    placeholder="Card title..."
                    value={newTitle}
                    onChange={e => setNewTitle(e.target.value)}
                    onKeyDown={e => e.key === "Enter" && addCard(col)}
                    autoFocus
                  />
                  <div style={{ display: "flex", gap: 4, marginTop: 4 }}>
                    <button style={btnPrimaryStyle} onClick={() => addCard(col)}>Add</button>
                    <button style={btnStyle} onClick={() => { setAddingTo(null); setNewTitle(""); }}>Cancel</button>
                  </div>
                </div>
              ) : (
                <button style={{ ...btnStyle, width: "100%", marginTop: 6, fontSize: 12 }} onClick={() => setAddingTo(col)}>
                  + Add Card
                </button>
              )}
            </div>
          );
        })}
      </div>

      {/* Card edit modal */}
      {editingCard && (
        <div style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.5)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 999 }} onClick={() => setEditingCard(null)}>
          <div style={{ background: "var(--bg-primary)", borderRadius: "var(--radius-md)", padding: 20, width: 460, maxHeight: "80vh", overflowY: "auto", border: "1px solid var(--border-color)", boxShadow: "var(--elevation-2)" }} onClick={e => e.stopPropagation()}>
            <h3 style={{ margin: "0 0 12px", color: "var(--text-primary)", fontSize: 16 }}>Edit Card</h3>
            <label style={{ fontSize: 12, color: "var(--text-secondary)" }}>Title</label>
            <input style={{ ...inputStyle, marginBottom: 8 }} value={editingCard.title} onChange={e => setEditingCard({ ...editingCard, title: e.target.value })} />
            <label style={{ fontSize: 12, color: "var(--text-secondary)" }}>Description</label>
            <textarea style={{ ...inputStyle, minHeight: 60, marginBottom: 8, resize: "vertical" }} value={editingCard.description} onChange={e => setEditingCard({ ...editingCard, description: e.target.value })} />
            <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
              <div style={{ flex: 1 }}>
                <label style={{ fontSize: 12, color: "var(--text-secondary)" }}>Assignee</label>
                <input style={inputStyle} value={editingCard.assignee} onChange={e => setEditingCard({ ...editingCard, assignee: e.target.value })} />
              </div>
              <div style={{ flex: 1 }}>
                <label style={{ fontSize: 12, color: "var(--text-secondary)" }}>Priority</label>
                <select style={inputStyle} value={editingCard.priority} onChange={e => setEditingCard({ ...editingCard, priority: e.target.value as Priority })}>
                  {PRIORITIES.map(p => <option key={p} value={p}>{p}</option>)}
                </select>
              </div>
              <div style={{ width: 80 }}>
                <label style={{ fontSize: 12, color: "var(--text-secondary)" }}>Points</label>
                <input style={inputStyle} type="number" min={0} value={editingCard.storyPoints} onChange={e => setEditingCard({ ...editingCard, storyPoints: Number(e.target.value) })} />
              </div>
            </div>
            <label style={{ fontSize: 12, color: "var(--text-secondary)" }}>Labels (comma-separated)</label>
            <input style={{ ...inputStyle, marginBottom: 8 }} value={editingCard.labels.join(", ")} onChange={e => setEditingCard({ ...editingCard, labels: e.target.value.split(",").map(s => s.trim()).filter(Boolean) })} />
            <label style={{ fontSize: 12, color: "var(--text-secondary)" }}>Column</label>
            <select style={{ ...inputStyle, marginBottom: 12 }} value={editingCard.column} onChange={e => setEditingCard({ ...editingCard, column: e.target.value as Column })}>
              {COLUMNS.map(c => <option key={c} value={c}>{c}</option>)}
            </select>
            <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
              <button style={btnStyle} onClick={() => setEditingCard(null)}>Cancel</button>
              <button style={btnPrimaryStyle} onClick={() => saveCard(editingCard)}>Save</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════════════════
   Sprint Tab
   ═══════════════════════════════════════════════════════════════════════ */

function SprintTab() {
  const [sprints, setSprints] = useState<Sprint[]>([]);
  const [history, setHistory] = useState<SprintHistory[]>([]);
  const [current, setCurrent] = useState<Sprint | null>(null);
  const [creating, setCreating] = useState(false);
  const [newSprint, setNewSprint] = useState({ name: "", goal: "", startDate: "", endDate: "" });
  const [error, setError] = useState("");

  useEffect(() => {
    (async () => {
      try {
        const data = await invoke<{ sprints: Sprint[]; history: SprintHistory[] }>("agile_get_sprints");
        setSprints(data.sprints || []);
        setHistory(data.history || []);
        const active = (data.sprints || []).find(s => s.status === "Active");
        if (active) setCurrent(active);
        else if ((data.sprints || []).length > 0) setCurrent(data.sprints[0]);
      } catch (e: any) {
        setError(typeof e === "string" ? e : e?.message || "Failed to load sprints");
      }
    })();
  }, []);

  const createSprint = useCallback(async () => {
    if (!newSprint.name.trim()) return;
    const sprint: Sprint = {
      id: genId(),
      name: newSprint.name,
      goal: newSprint.goal,
      startDate: newSprint.startDate || new Date().toISOString().slice(0, 10),
      endDate: newSprint.endDate || "",
      status: "Planning",
      velocity: 0,
      plannedPoints: 0,
      completedPoints: 0,
      cards: [],
    };
    try {
      await invoke("agile_create_sprint", { sprint });
      setSprints(prev => [...prev, sprint]);
      setCurrent(sprint);
      setCreating(false);
      setNewSprint({ name: "", goal: "", startDate: "", endDate: "" });
    } catch (e: any) {
      setError(typeof e === "string" ? e : e?.message || "Failed to create sprint");
    }
  }, [newSprint]);

  const updateSprintStatus = useCallback(async (status: SprintStatus) => {
    if (!current) return;
    const updated = { ...current, status };
    try {
      await invoke("agile_update_sprint", { sprint: updated });
      setCurrent(updated);
      setSprints(prev => prev.map(s => s.id === updated.id ? updated : s));
    } catch (e: any) {
      setError(typeof e === "string" ? e : e?.message || "Failed to update sprint");
    }
  }, [current]);

  /* Simple text burndown */
  const renderBurndown = () => {
    if (!current) return null;
    const total = current.plannedPoints || 20;
    const days = 10;
    const lines: string[] = [];
    for (let d = 0; d <= days; d++) {
      const idealRemain = Math.round(total * (1 - d / days));
      const actualRemain = Math.round(total * (1 - (d / days) * (current.completedPoints / Math.max(total, 1))));
      const idealBar = "=".repeat(Math.max(idealRemain, 0));
      const actualBar = "#".repeat(Math.max(actualRemain, 0));
      lines.push(`Day ${String(d).padStart(2, " ")} | Ideal: ${idealBar.padEnd(total, " ")} ${idealRemain}`);
      lines.push(`       | Actual: ${actualBar.padEnd(total, " ")} ${actualRemain}`);
    }
    return lines.join("\n");
  };

  return (
    <div>
      {error && <div style={{ color: "var(--error-color)", marginBottom: 8, fontSize: 12 }}>{error}</div>}

      {/* Sprint selector */}
      <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 12 }}>
        <select style={{ ...inputStyle, width: "auto" }} value={current?.id || ""} onChange={e => { const s = sprints.find(x => x.id === e.target.value); if (s) setCurrent(s); }}>
          <option value="">Select Sprint</option>
          {sprints.map(s => <option key={s.id} value={s.id}>{s.name} ({s.status})</option>)}
        </select>
        <button style={btnPrimaryStyle} onClick={() => setCreating(true)}>+ New Sprint</button>
        {current && current.status === "Planning" && <button style={{ ...btnStyle, background: "var(--accent-green)", color: "#fff" }} onClick={() => updateSprintStatus("Active")}>Start Sprint</button>}
        {current && current.status === "Active" && <button style={{ ...btnStyle, background: "var(--accent-rose)", color: "#fff" }} onClick={() => updateSprintStatus("Completed")}>End Sprint</button>}
      </div>

      {/* Create sprint form */}
      {creating && (
        <div style={{ ...cardBaseStyle, marginBottom: 12 }}>
          <h4 style={{ margin: "0 0 8px", color: "var(--text-primary)" }}>New Sprint</h4>
          <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
            <input style={{ ...inputStyle, flex: 2 }} placeholder="Sprint name" value={newSprint.name} onChange={e => setNewSprint({ ...newSprint, name: e.target.value })} />
            <input style={{ ...inputStyle, flex: 1 }} type="date" value={newSprint.startDate} onChange={e => setNewSprint({ ...newSprint, startDate: e.target.value })} />
            <input style={{ ...inputStyle, flex: 1 }} type="date" value={newSprint.endDate} onChange={e => setNewSprint({ ...newSprint, endDate: e.target.value })} />
          </div>
          <input style={{ ...inputStyle, marginBottom: 8 }} placeholder="Sprint goal" value={newSprint.goal} onChange={e => setNewSprint({ ...newSprint, goal: e.target.value })} />
          <div style={{ display: "flex", gap: 8 }}>
            <button style={btnPrimaryStyle} onClick={createSprint}>Create</button>
            <button style={btnStyle} onClick={() => setCreating(false)}>Cancel</button>
          </div>
        </div>
      )}

      {/* Current sprint info */}
      {current && (
        <div>
          <div style={{ display: "flex", gap: 16, marginBottom: 12, flexWrap: "wrap" }}>
            {[
              { label: "Goal", value: current.goal || "Not set" },
              { label: "Status", value: current.status },
              { label: "Dates", value: `${current.startDate} - ${current.endDate || "TBD"}` },
              { label: "Velocity", value: String(current.velocity) },
              { label: "Planned", value: `${current.plannedPoints} pts` },
              { label: "Completed", value: `${current.completedPoints} pts` },
            ].map(({ label, value }) => (
              <div key={label} style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", padding: "8px 14px", border: "1px solid var(--border-color)" }}>
                <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>{label}</div>
                <div style={{ fontSize: 14, fontWeight: 600, color: "var(--text-primary)" }}>{value}</div>
              </div>
            ))}
          </div>

          {/* Sprint backlog table */}
          <div style={sectionTitle}>Sprint Backlog</div>
          <div style={{ overflowX: "auto" }}>
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 13 }}>
              <thead>
                <tr style={{ borderBottom: "1px solid var(--border-color)" }}>
                  {["Story", "Points", "Assignee", "Status", "Priority"].map(h => (
                    <th key={h} style={{ textAlign: "left", padding: "6px 10px", color: "var(--text-secondary)", fontWeight: 500, fontSize: 12 }}>{h}</th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {(current.cards || []).map(c => (
                  <tr key={c.id} style={{ borderBottom: "1px solid var(--border-color)" }}>
                    <td style={{ padding: "6px 10px", color: "var(--text-primary)" }}>{c.title}</td>
                    <td style={{ padding: "6px 10px" }}><span style={badgeStyle("var(--accent-purple)")}>{c.storyPoints}</span></td>
                    <td style={{ padding: "6px 10px", color: "var(--text-secondary)" }}>{c.assignee || "-"}</td>
                    <td style={{ padding: "6px 10px" }}><span style={badgeStyle("var(--accent-blue)")}>{c.column}</span></td>
                    <td style={{ padding: "6px 10px" }}><span style={badgeStyle(PRIORITY_COLORS[c.priority])}>{c.priority}</span></td>
                  </tr>
                ))}
                {(current.cards || []).length === 0 && (
                  <tr><td colSpan={5} style={{ padding: 16, textAlign: "center", color: "var(--text-secondary)" }}>No stories in this sprint</td></tr>
                )}
              </tbody>
            </table>
          </div>

          {/* Burndown */}
          <div style={{ ...sectionTitle, marginTop: 16 }}>Burndown Chart</div>
          <pre style={{ background: "var(--bg-secondary)", padding: 12, borderRadius: "var(--radius-sm)", fontSize: 11, overflow: "auto", color: "var(--text-primary)", border: "1px solid var(--border-color)" }}>
            {renderBurndown()}
          </pre>
        </div>
      )}

      {/* Sprint history */}
      {history.length > 0 && (
        <div style={{ marginTop: 16 }}>
          <div style={sectionTitle}>Sprint History</div>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
            {history.map(h => (
              <div key={h.id} style={{ ...cardBaseStyle, minWidth: 160 }}>
                <div style={{ fontWeight: 600, fontSize: 13, color: "var(--text-primary)" }}>{h.name}</div>
                <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Velocity: {h.velocity} | {h.completedPoints}/{h.plannedPoints} pts</div>
                <span style={badgeStyle(h.status === "Completed" ? "var(--success-color)" : "var(--text-secondary)")}>{h.status}</span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════════════════
   Backlog Tab
   ═══════════════════════════════════════════════════════════════════════ */

function BacklogTab() {
  const [items, setItems] = useState<Card[]>([]);
  const [filterPriority, setFilterPriority] = useState<Priority | "">("");
  const [filterLabel, setFilterLabel] = useState("");
  const [filterAssignee, setFilterAssignee] = useState("");
  const [showCreate, setShowCreate] = useState(false);
  const [newStory, setNewStory] = useState({ title: "", description: "", storyPoints: 0, priority: "P2" as Priority, labels: "", acceptanceCriteria: "" });
  const [error, setError] = useState("");

  useEffect(() => {
    (async () => {
      try {
        const data = await invoke<Card[]>("agile_get_backlog");
        setItems(data || []);
      } catch (e: any) {
        setError(typeof e === "string" ? e : e?.message || "Failed to load backlog");
      }
    })();
  }, []);

  const createStory = useCallback(async () => {
    if (!newStory.title.trim()) return;
    const story: Card = {
      id: genId(),
      title: newStory.title,
      description: newStory.description,
      assignee: "",
      priority: newStory.priority,
      storyPoints: newStory.storyPoints,
      labels: newStory.labels.split(",").map(s => s.trim()).filter(Boolean),
      column: "Backlog",
      acceptanceCriteria: newStory.acceptanceCriteria.split("\n").filter(Boolean),
      createdAt: new Date().toISOString(),
    };
    try {
      await invoke("agile_create_story", { story });
      setItems(prev => [story, ...prev]);
      setNewStory({ title: "", description: "", storyPoints: 0, priority: "P2", labels: "", acceptanceCriteria: "" });
      setShowCreate(false);
    } catch (e: any) {
      setError(typeof e === "string" ? e : e?.message || "Failed to create story");
    }
  }, [newStory]);

  const updateInline = useCallback(async (id: string, field: "storyPoints" | "priority", value: any) => {
    const item = items.find(c => c.id === id);
    if (!item) return;
    const updated = { ...item, [field]: value };
    try {
      await invoke("agile_update_story", { story: updated });
      setItems(prev => prev.map(c => c.id === id ? updated : c));
    } catch (e: any) {
      setError(typeof e === "string" ? e : e?.message || "Failed to update story");
    }
  }, [items]);

  const suggestSplit = useCallback(async (id: string) => {
    const item = items.find(c => c.id === id);
    if (!item) return;
    // AI-powered suggestion: split large stories
    if (item.storyPoints > 5) {
      const half = Math.ceil(item.storyPoints / 2);
      const childA: Card = { ...item, id: genId(), title: `${item.title} (Part 1)`, storyPoints: half };
      const childB: Card = { ...item, id: genId(), title: `${item.title} (Part 2)`, storyPoints: item.storyPoints - half };
      try {
        await invoke("agile_create_story", { story: childA });
        await invoke("agile_create_story", { story: childB });
        setItems(prev => [childA, childB, ...prev.filter(c => c.id !== id)]);
      } catch (e: any) {
        setError(typeof e === "string" ? e : e?.message || "Failed to split story");
      }
    } else {
      setError("Story is already small enough (<=5 pts). No split needed.");
      setTimeout(() => setError(""), 3000);
    }
  }, [items]);

  const filtered = items.filter(c => {
    if (filterPriority && c.priority !== filterPriority) return false;
    if (filterLabel && !c.labels.some(l => l.toLowerCase().includes(filterLabel.toLowerCase()))) return false;
    if (filterAssignee && !c.assignee.toLowerCase().includes(filterAssignee.toLowerCase())) return false;
    return true;
  });

  return (
    <div>
      {error && <div style={{ color: "var(--error-color)", marginBottom: 8, fontSize: 12 }}>{error}</div>}

      {/* Create story form */}
      <div style={{ marginBottom: 12 }}>
        {showCreate ? (
          <div style={{ ...cardBaseStyle }}>
            <h4 style={{ margin: "0 0 8px", color: "var(--text-primary)" }}>Create Story</h4>
            <input style={{ ...inputStyle, marginBottom: 6 }} placeholder="Title" value={newStory.title} onChange={e => setNewStory({ ...newStory, title: e.target.value })} />
            <textarea style={{ ...inputStyle, marginBottom: 6, minHeight: 50, resize: "vertical" }} placeholder="Description" value={newStory.description} onChange={e => setNewStory({ ...newStory, description: e.target.value })} />
            <div style={{ display: "flex", gap: 8, marginBottom: 6 }}>
              <select style={{ ...inputStyle, width: "auto" }} value={newStory.priority} onChange={e => setNewStory({ ...newStory, priority: e.target.value as Priority })}>
                {PRIORITIES.map(p => <option key={p} value={p}>{p}</option>)}
              </select>
              <input style={{ ...inputStyle, width: 80 }} type="number" min={0} placeholder="Points" value={newStory.storyPoints || ""} onChange={e => setNewStory({ ...newStory, storyPoints: Number(e.target.value) })} />
              <input style={{ ...inputStyle, flex: 1 }} placeholder="Labels (comma-separated)" value={newStory.labels} onChange={e => setNewStory({ ...newStory, labels: e.target.value })} />
            </div>
            <textarea style={{ ...inputStyle, marginBottom: 8, minHeight: 40, resize: "vertical" }} placeholder="Acceptance criteria (one per line)" value={newStory.acceptanceCriteria} onChange={e => setNewStory({ ...newStory, acceptanceCriteria: e.target.value })} />
            <div style={{ display: "flex", gap: 8 }}>
              <button style={btnPrimaryStyle} onClick={createStory}>Create</button>
              <button style={btnStyle} onClick={() => setShowCreate(false)}>Cancel</button>
            </div>
          </div>
        ) : (
          <button style={btnPrimaryStyle} onClick={() => setShowCreate(true)}>+ Create Story</button>
        )}
      </div>

      {/* Filters */}
      <div style={{ display: "flex", gap: 8, marginBottom: 12, flexWrap: "wrap", alignItems: "center" }}>
        <span style={{ fontSize: 12, color: "var(--text-secondary)" }}>Filter:</span>
        <select style={{ ...inputStyle, width: "auto" }} value={filterPriority} onChange={e => setFilterPriority(e.target.value as Priority | "")}>
          <option value="">All Priorities</option>
          {PRIORITIES.map(p => <option key={p} value={p}>{p}</option>)}
        </select>
        <input style={{ ...inputStyle, width: 140 }} placeholder="Label" value={filterLabel} onChange={e => setFilterLabel(e.target.value)} />
        <input style={{ ...inputStyle, width: 140 }} placeholder="Assignee" value={filterAssignee} onChange={e => setFilterAssignee(e.target.value)} />
      </div>

      {/* Backlog list */}
      {filtered.map(item => (
        <div key={item.id} style={{ ...cardBaseStyle, display: "flex", alignItems: "center", gap: 12 }}>
          <div style={{ flex: 1 }}>
            <div style={{ fontWeight: 500, fontSize: 13, color: "var(--text-primary)" }}>{item.title}</div>
            {item.description && <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 2 }}>{item.description.slice(0, 100)}{item.description.length > 100 ? "..." : ""}</div>}
            <div style={{ display: "flex", gap: 4, marginTop: 4, flexWrap: "wrap" }}>
              {item.labels.map(l => <span key={l} style={badgeStyle("var(--bg-tertiary)", "var(--text-secondary)")}>{l}</span>)}
              {item.acceptanceCriteria.length > 0 && <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>AC: {item.acceptanceCriteria.length}</span>}
            </div>
          </div>
          <select
            style={{ ...inputStyle, width: 60 }}
            value={item.priority}
            onChange={e => updateInline(item.id, "priority", e.target.value)}
            onClick={e => e.stopPropagation()}
          >
            {PRIORITIES.map(p => <option key={p} value={p}>{p}</option>)}
          </select>
          <input
            style={{ ...inputStyle, width: 55, textAlign: "center" }}
            type="number"
            min={0}
            value={item.storyPoints}
            onChange={e => updateInline(item.id, "storyPoints", Number(e.target.value))}
            onClick={e => e.stopPropagation()}
          />
          <button style={{ ...btnStyle, fontSize: 11, padding: "4px 8px" }} title="AI Split Suggestion" onClick={() => suggestSplit(item.id)}>Split</button>
        </div>
      ))}
      {filtered.length === 0 && <div style={{ textAlign: "center", color: "var(--text-secondary)", padding: 24 }}>No backlog items found</div>}
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════════════════
   Ceremonies Tab
   ═══════════════════════════════════════════════════════════════════════ */

function CeremoniesTab() {
  const [subTab, setSubTab] = useState<"standup" | "planning" | "review" | "retro">("standup");
  const [standups, setStandups] = useState<StandupEntry[]>([]);
  const [capacity, setCapacity] = useState({ members: 5, days: 10, focusFactor: 0.7 });
  const [demoChecklist, setDemoChecklist] = useState<{ item: string; done: boolean }[]>([]);
  const [retro, setRetro] = useState<RetroCard[]>([]);
  const [newStandup, setNewStandup] = useState<StandupEntry>({ member: "", didYesterday: "", willDoToday: "", blockers: "" });
  const [newRetroText, setNewRetroText] = useState("");
  const [newDemoItem, setNewDemoItem] = useState("");
  const [error, setError] = useState("");

  useEffect(() => {
    (async () => {
      try {
        const data = await invoke<CeremonyData>("agile_get_ceremonies");
        if (data.standups) setStandups(data.standups);
        if (data.capacity) setCapacity(data.capacity);
        if (data.demoChecklist) setDemoChecklist(data.demoChecklist);
        if (data.retro) setRetro(data.retro);
      } catch (e: any) {
        setError(typeof e === "string" ? e : e?.message || "Failed to load ceremonies");
      }
    })();
  }, []);

  const saveCeremony = useCallback(async (data: Partial<CeremonyData>) => {
    try {
      const ceremony = {
        standups: data.standups ?? standups,
        capacity: data.capacity ?? capacity,
        demoChecklist: data.demoChecklist ?? demoChecklist,
        retro: data.retro ?? retro,
      };
      await invoke("agile_save_ceremony", { ceremony });
    } catch (e: any) {
      setError(typeof e === "string" ? e : e?.message || "Failed to save ceremony");
    }
  }, [standups, capacity, demoChecklist, retro]);

  const addStandup = () => {
    if (!newStandup.member.trim()) return;
    const next = [...standups, { ...newStandup }];
    setStandups(next);
    setNewStandup({ member: "", didYesterday: "", willDoToday: "", blockers: "" });
    saveCeremony({ standups: next });
  };

  const addRetroCard = (category: RetroCard["category"]) => {
    if (!newRetroText.trim()) return;
    const card: RetroCard = { id: genId(), text: newRetroText.trim(), category };
    const next = [...retro, card];
    setRetro(next);
    setNewRetroText("");
    saveCeremony({ retro: next });
  };

  const toggleDemo = (idx: number) => {
    const next = demoChecklist.map((d, i) => i === idx ? { ...d, done: !d.done } : d);
    setDemoChecklist(next);
    saveCeremony({ demoChecklist: next });
  };

  const addDemoItem = () => {
    if (!newDemoItem.trim()) return;
    const next = [...demoChecklist, { item: newDemoItem.trim(), done: false }];
    setDemoChecklist(next);
    setNewDemoItem("");
    saveCeremony({ demoChecklist: next });
  };

  const subTabBtn = (key: typeof subTab, label: string) => (
    <button style={{ ...btnStyle, background: subTab === key ? "var(--accent-blue)" : "var(--bg-elevated)", color: subTab === key ? "#fff" : "var(--text-primary)" }} onClick={() => setSubTab(key)}>{label}</button>
  );

  return (
    <div>
      {error && <div style={{ color: "var(--error-color)", marginBottom: 8, fontSize: 12 }}>{error}</div>}
      <div style={{ display: "flex", gap: 6, marginBottom: 14 }}>
        {subTabBtn("standup", "Daily Standup")}
        {subTabBtn("planning", "Sprint Planning")}
        {subTabBtn("review", "Sprint Review")}
        {subTabBtn("retro", "Retrospective")}
      </div>

      {/* Daily Standup */}
      {subTab === "standup" && (
        <div>
          <div style={sectionTitle}>Daily Standup</div>
          {standups.map((s, i) => (
            <div key={i} style={{ ...cardBaseStyle }}>
              <div style={{ fontWeight: 600, fontSize: 13, color: "var(--accent-blue)", marginBottom: 4 }}>{s.member}</div>
              <div style={{ fontSize: 12, color: "var(--text-primary)" }}><strong>Did:</strong> {s.didYesterday}</div>
              <div style={{ fontSize: 12, color: "var(--text-primary)" }}><strong>Will do:</strong> {s.willDoToday}</div>
              {s.blockers && <div style={{ fontSize: 12, color: "var(--error-color)" }}><strong>Blockers:</strong> {s.blockers}</div>}
            </div>
          ))}
          <div style={{ ...cardBaseStyle, background: "var(--bg-secondary)" }}>
            <input style={{ ...inputStyle, marginBottom: 4 }} placeholder="Team member" value={newStandup.member} onChange={e => setNewStandup({ ...newStandup, member: e.target.value })} />
            <input style={{ ...inputStyle, marginBottom: 4 }} placeholder="What I did yesterday" value={newStandup.didYesterday} onChange={e => setNewStandup({ ...newStandup, didYesterday: e.target.value })} />
            <input style={{ ...inputStyle, marginBottom: 4 }} placeholder="What I'll do today" value={newStandup.willDoToday} onChange={e => setNewStandup({ ...newStandup, willDoToday: e.target.value })} />
            <input style={{ ...inputStyle, marginBottom: 6 }} placeholder="Blockers (if any)" value={newStandup.blockers} onChange={e => setNewStandup({ ...newStandup, blockers: e.target.value })} />
            <button style={btnPrimaryStyle} onClick={addStandup}>Add Entry</button>
          </div>
        </div>
      )}

      {/* Sprint Planning */}
      {subTab === "planning" && (
        <div>
          <div style={sectionTitle}>Capacity Calculator</div>
          <div style={{ display: "flex", gap: 12, marginBottom: 12, flexWrap: "wrap" }}>
            <div>
              <label style={{ fontSize: 12, color: "var(--text-secondary)" }}>Team Members</label>
              <input style={{ ...inputStyle, width: 80 }} type="number" min={1} value={capacity.members} onChange={e => { const v = { ...capacity, members: Number(e.target.value) }; setCapacity(v); saveCeremony({ capacity: v }); }} />
            </div>
            <div>
              <label style={{ fontSize: 12, color: "var(--text-secondary)" }}>Available Days</label>
              <input style={{ ...inputStyle, width: 80 }} type="number" min={1} value={capacity.days} onChange={e => { const v = { ...capacity, days: Number(e.target.value) }; setCapacity(v); saveCeremony({ capacity: v }); }} />
            </div>
            <div>
              <label style={{ fontSize: 12, color: "var(--text-secondary)" }}>Focus Factor</label>
              <input style={{ ...inputStyle, width: 80 }} type="number" step={0.05} min={0} max={1} value={capacity.focusFactor} onChange={e => { const v = { ...capacity, focusFactor: Number(e.target.value) }; setCapacity(v); saveCeremony({ capacity: v }); }} />
            </div>
          </div>
          <div style={{ ...cardBaseStyle, background: "var(--bg-secondary)" }}>
            <div style={{ fontSize: 14, fontWeight: 600, color: "var(--accent-green)" }}>
              Total Capacity: {(capacity.members * capacity.days * capacity.focusFactor).toFixed(1)} person-days
            </div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 4 }}>
              {capacity.members} members x {capacity.days} days x {(capacity.focusFactor * 100).toFixed(0)}% focus
            </div>
          </div>
        </div>
      )}

      {/* Sprint Review */}
      {subTab === "review" && (
        <div>
          <div style={sectionTitle}>Demo Checklist</div>
          {demoChecklist.map((d, i) => (
            <div key={i} style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6, cursor: "pointer" }} onClick={() => toggleDemo(i)}>
              <span style={{ fontSize: 16, color: d.done ? "var(--success-color)" : "var(--text-secondary)" }}>{d.done ? "[x]" : "[ ]"}</span>
              <span style={{ fontSize: 13, color: d.done ? "var(--text-secondary)" : "var(--text-primary)", textDecoration: d.done ? "line-through" : "none" }}>{d.item}</span>
            </div>
          ))}
          <div style={{ display: "flex", gap: 6, marginTop: 8 }}>
            <input style={{ ...inputStyle, flex: 1 }} placeholder="Add demo item..." value={newDemoItem} onChange={e => setNewDemoItem(e.target.value)} onKeyDown={e => e.key === "Enter" && addDemoItem()} />
            <button style={btnPrimaryStyle} onClick={addDemoItem}>Add</button>
          </div>
        </div>
      )}

      {/* Retrospective */}
      {subTab === "retro" && (
        <div>
          <div style={sectionTitle}>Retrospective</div>
          <div style={{ display: "flex", gap: 12, marginBottom: 12 }}>
            <input style={{ ...inputStyle, flex: 1 }} placeholder="Add a card..." value={newRetroText} onChange={e => setNewRetroText(e.target.value)} />
          </div>
          <div style={{ display: "flex", gap: 12 }}>
            {(["well", "didnt", "action"] as const).map(cat => {
              const title = cat === "well" ? "What went well" : cat === "didnt" ? "What didn't go well" : "Action items";
              const color = cat === "well" ? "var(--success-color)" : cat === "didnt" ? "var(--error-color)" : "var(--accent-blue)";
              return (
                <div key={cat} style={{ flex: 1, background: "var(--bg-secondary)", borderRadius: "var(--radius-md)", padding: 10, border: "1px solid var(--border-color)" }}>
                  <div style={{ fontWeight: 600, fontSize: 13, color, marginBottom: 8, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                    {title}
                    <button style={{ ...btnStyle, padding: "2px 8px", fontSize: 11 }} onClick={() => addRetroCard(cat)}>+</button>
                  </div>
                  {retro.filter(r => r.category === cat).map(r => (
                    <div key={r.id} style={{ ...cardBaseStyle, fontSize: 12 }}>{r.text}</div>
                  ))}
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════════════════
   Metrics Tab
   ═══════════════════════════════════════════════════════════════════════ */

function MetricsTab() {
  const [metrics, setMetrics] = useState<MetricsData | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    (async () => {
      try {
        const data = await invoke<MetricsData>("agile_get_metrics");
        setMetrics(data);
      } catch (e: any) {
        setError(typeof e === "string" ? e : e?.message || "Failed to load metrics");
      }
    })();
  }, []);

  if (!metrics && !error) return <div style={{ color: "var(--text-secondary)", padding: 24, textAlign: "center" }}>Loading metrics...</div>;
  if (error) return <div style={{ color: "var(--error-color)", padding: 24 }}>{error}</div>;
  if (!metrics) return null;

  const maxVel = Math.max(...metrics.velocityHistory.map(v => v.points), 1);

  return (
    <div>
      {/* KPI cards */}
      <div style={{ display: "flex", gap: 12, marginBottom: 16, flexWrap: "wrap" }}>
        {[
          { label: "Cycle Time", value: `${metrics.cycleTimeDays.toFixed(1)} days`, color: "var(--accent-blue)" },
          { label: "Lead Time", value: `${metrics.leadTimeDays.toFixed(1)} days`, color: "var(--accent-purple)" },
          { label: "Plan vs Done", value: `${(metrics.plannedVsCompleted * 100).toFixed(0)}%`, color: metrics.plannedVsCompleted >= 0.8 ? "var(--success-color)" : "var(--warning-color)" },
          { label: "Scope Creep", value: `${metrics.scopeCreepPct.toFixed(0)}%`, color: metrics.scopeCreepPct > 20 ? "var(--error-color)" : "var(--success-color)" },
          { label: "Capacity", value: `${(metrics.capacityUtilization * 100).toFixed(0)}%`, color: "var(--accent-gold)" },
        ].map(({ label, value, color }) => (
          <div key={label} style={{ background: "var(--bg-elevated)", borderRadius: "var(--radius-md)", padding: "12px 18px", border: "1px solid var(--border-color)", minWidth: 130, boxShadow: "var(--card-shadow)" }}>
            <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>{label}</div>
            <div style={{ fontSize: 20, fontWeight: 700, color }}>{value}</div>
          </div>
        ))}
      </div>

      {/* Velocity chart */}
      <div style={sectionTitle}>Velocity (last {metrics.velocityHistory.length} sprints)</div>
      <div style={{ display: "flex", gap: 8, alignItems: "flex-end", height: 120, marginBottom: 16, padding: "0 4px" }}>
        {metrics.velocityHistory.map(v => (
          <div key={v.sprint} style={{ flex: 1, display: "flex", flexDirection: "column", alignItems: "center" }}>
            <div style={{ fontSize: 11, fontWeight: 600, color: "var(--text-primary)", marginBottom: 2 }}>{v.points}</div>
            <div style={{
              width: "100%",
              maxWidth: 48,
              height: `${(v.points / maxVel) * 100}px`,
              background: "var(--accent-blue)",
              borderRadius: "var(--radius-sm) var(--radius-sm) 0 0",
              transition: "var(--transition-smooth)",
            }} />
            <div style={{ fontSize: 10, color: "var(--text-secondary)", marginTop: 4 }}>{v.sprint}</div>
          </div>
        ))}
      </div>

      {/* Cumulative flow */}
      <div style={sectionTitle}>Cumulative Flow</div>
      <div style={{ marginBottom: 16 }}>
        {metrics.cumulativeFlow.slice(-6).map(row => {
          const total = row.backlog + row.todo + row.inProgress + row.inReview + row.done;
          const pct = (v: number) => total > 0 ? `${(v / total * 100).toFixed(0)}%` : "0%";
          return (
            <div key={row.date} style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
              <span style={{ width: 60, fontSize: 11, color: "var(--text-secondary)" }}>{row.date}</span>
              <div style={{ flex: 1, display: "flex", height: 18, borderRadius: "var(--radius-sm)", overflow: "hidden" }}>
                <div style={{ width: pct(row.backlog), background: "#6b7280", transition: "var(--transition-smooth)" }} title={`Backlog: ${row.backlog}`} />
                <div style={{ width: pct(row.todo), background: "#3b82f6", transition: "var(--transition-smooth)" }} title={`To Do: ${row.todo}`} />
                <div style={{ width: pct(row.inProgress), background: "#f59e0b", transition: "var(--transition-smooth)" }} title={`In Progress: ${row.inProgress}`} />
                <div style={{ width: pct(row.inReview), background: "#8b5cf6", transition: "var(--transition-smooth)" }} title={`In Review: ${row.inReview}`} />
                <div style={{ width: pct(row.done), background: "#10b981", transition: "var(--transition-smooth)" }} title={`Done: ${row.done}`} />
              </div>
            </div>
          );
        })}
        <div style={{ display: "flex", gap: 12, marginTop: 6, fontSize: 11 }}>
          {[
            { label: "Backlog", color: "#6b7280" },
            { label: "To Do", color: "#3b82f6" },
            { label: "In Progress", color: "#f59e0b" },
            { label: "In Review", color: "#8b5cf6" },
            { label: "Done", color: "#10b981" },
          ].map(({ label, color }) => (
            <span key={label} style={{ display: "flex", alignItems: "center", gap: 4, color: "var(--text-secondary)" }}>
              <span style={{ width: 10, height: 10, borderRadius: 2, background: color, display: "inline-block" }} />
              {label}
            </span>
          ))}
        </div>
      </div>

      {/* Sprint health */}
      <div style={sectionTitle}>Sprint Health Indicators</div>
      <div style={{ display: "flex", gap: 12, flexWrap: "wrap" }}>
        <div style={{ ...cardBaseStyle, flex: 1, minWidth: 180 }}>
          <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Planned vs Completed Ratio</div>
          <div style={{ height: 8, background: "var(--bg-tertiary)", borderRadius: 4, marginTop: 6 }}>
            <div style={{ height: "100%", width: `${Math.min(metrics.plannedVsCompleted * 100, 100)}%`, background: metrics.plannedVsCompleted >= 0.8 ? "var(--success-color)" : "var(--warning-color)", borderRadius: 4, transition: "var(--transition-smooth)" }} />
          </div>
        </div>
        <div style={{ ...cardBaseStyle, flex: 1, minWidth: 180 }}>
          <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Scope Creep</div>
          <div style={{ height: 8, background: "var(--bg-tertiary)", borderRadius: 4, marginTop: 6 }}>
            <div style={{ height: "100%", width: `${Math.min(metrics.scopeCreepPct, 100)}%`, background: metrics.scopeCreepPct > 20 ? "var(--error-color)" : "var(--success-color)", borderRadius: 4, transition: "var(--transition-smooth)" }} />
          </div>
        </div>
      </div>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════════════════
   Methodology Tab
   ═══════════════════════════════════════════════════════════════════════ */

interface MethodologyInfo {
  name: Methodology;
  description: string;
  principles: string[];
  roles: string[];
  ceremonies: string[];
  artifacts: string[];
  bestFor: string;
}

const METHODOLOGIES: MethodologyInfo[] = [
  {
    name: "Scrum",
    description: "An iterative and incremental agile framework for managing product development. Work is organized into fixed-length sprints (1-4 weeks) with defined roles and ceremonies.",
    principles: ["Transparency", "Inspection", "Adaptation", "Empirical process control", "Self-organization", "Time-boxing"],
    roles: ["Product Owner", "Scrum Master", "Development Team"],
    ceremonies: ["Sprint Planning", "Daily Scrum", "Sprint Review", "Sprint Retrospective", "Backlog Refinement"],
    artifacts: ["Product Backlog", "Sprint Backlog", "Increment", "Definition of Done"],
    bestFor: "Teams of 5-9, complex products with evolving requirements, 2-4 week delivery cycles",
  },
  {
    name: "Kanban",
    description: "A visual method for managing work as it moves through a process. Focuses on continuous delivery without fixed iterations, limiting work-in-progress to maximize flow.",
    principles: ["Visualize workflow", "Limit WIP", "Manage flow", "Make policies explicit", "Implement feedback loops", "Improve collaboratively"],
    roles: ["No prescribed roles (existing roles continue)", "Service Delivery Manager (optional)"],
    ceremonies: ["Replenishment Meeting", "Delivery Planning", "Service Delivery Review", "Operations Review"],
    artifacts: ["Kanban Board", "WIP Limits", "Cumulative Flow Diagram", "Lead Time Distribution"],
    bestFor: "Support/maintenance teams, continuous delivery, teams wanting minimal process overhead",
  },
  {
    name: "XP",
    description: "Extreme Programming emphasizes technical excellence and frequent releases in short development cycles. Strong focus on engineering practices and customer satisfaction.",
    principles: ["Communication", "Simplicity", "Feedback", "Courage", "Respect", "Continuous improvement"],
    roles: ["Customer", "Developer", "Tracker", "Coach"],
    ceremonies: ["Iteration Planning", "Stand-up", "Iteration Demo", "Retrospective"],
    artifacts: ["User Stories", "Release Plan", "Iteration Plan", "Acceptance Tests", "Code (with tests)"],
    bestFor: "Small teams (2-12), projects requiring high code quality, rapidly changing requirements",
  },
  {
    name: "Lean",
    description: "Adapted from Toyota Production System, Lean focuses on delivering maximum value while minimizing waste. Emphasizes flow efficiency over resource efficiency.",
    principles: ["Eliminate waste", "Amplify learning", "Decide late", "Deliver fast", "Empower the team", "Build integrity in", "See the whole"],
    roles: ["No prescribed roles", "Value Stream Manager (optional)"],
    ceremonies: ["Value Stream Mapping", "Kaizen Events", "Gemba Walks"],
    artifacts: ["Value Stream Map", "A3 Problem-Solving Report", "Kanban Boards"],
    bestFor: "Organizations seeking to reduce waste, improve efficiency, any team size",
  },
  {
    name: "FDD",
    description: "Feature-Driven Development is a model-driven, short-iteration process. Work is organized around features (small, client-valued functions) delivered every 2 weeks.",
    principles: ["Domain object modeling", "Developing by feature", "Individual class ownership", "Feature teams", "Inspections", "Regular builds"],
    roles: ["Project Manager", "Chief Architect", "Development Manager", "Chief Programmer", "Class Owner", "Domain Expert"],
    ceremonies: ["Develop Overall Model", "Build Feature List", "Plan by Feature", "Design by Feature", "Build by Feature"],
    artifacts: ["Overall Model", "Feature List", "Feature Set", "Design Packages", "Class Diagrams"],
    bestFor: "Large teams (20+), enterprise projects, teams valuing design and modeling",
  },
  {
    name: "Crystal",
    description: "A family of methodologies (Clear, Yellow, Orange, Red) scaled by team size and criticality. Emphasizes people and interactions with minimal process.",
    principles: ["Frequent delivery", "Reflective improvement", "Osmotic communication", "Personal safety", "Focus", "Easy access to expert users"],
    roles: ["Executive Sponsor", "Lead Designer", "Designer-Programmer", "User Expert"],
    ceremonies: ["Reflection Workshop", "Blitz Planning", "Methodology Shaping"],
    artifacts: ["Release Plan", "Status Report", "Risk List", "Use Cases"],
    bestFor: "Teams of 2-50, projects where people and communication matter most, varying criticality levels",
  },
  {
    name: "SAFe",
    description: "Scaled Agile Framework provides a structured approach for scaling agile across large enterprises. Organizes work at Team, Program, Large Solution, and Portfolio levels.",
    principles: ["Take an economic view", "Apply systems thinking", "Assume variability", "Build incrementally", "Base milestones on evaluation", "Visualize WIP", "Reduce batch sizes", "Apply cadence", "Unlock intrinsic motivation", "Decentralize decision-making"],
    roles: ["Release Train Engineer", "Product Manager", "System Architect", "Epic Owner", "Scrum Master", "Product Owner", "Agile Team"],
    ceremonies: ["PI Planning", "System Demo", "ART Sync", "Inspect & Adapt", "Coach Sync", "PO Sync"],
    artifacts: ["Program Board", "PI Objectives", "Portfolio Kanban", "Solution Backlog", "Architectural Runway"],
    bestFor: "Large enterprises (50+ developers), multi-team coordination, regulated industries",
  },
];

function MethodologyTab() {
  const [selected, setSelected] = useState<Methodology>("Scrum");
  const [enabledPractices, setEnabledPractices] = useState<Record<string, boolean>>({});
  const [showCompare, setShowCompare] = useState(false);

  const info = METHODOLOGIES.find(m => m.name === selected)!;

  const togglePractice = (practice: string) => {
    setEnabledPractices(prev => ({ ...prev, [practice]: !prev[practice] }));
  };

  return (
    <div>
      {/* Methodology selector */}
      <div style={{ display: "flex", gap: 6, marginBottom: 14, flexWrap: "wrap", alignItems: "center" }}>
        {METHODOLOGIES.map(m => (
          <button key={m.name} style={{ ...btnStyle, background: selected === m.name ? "var(--accent-blue)" : "var(--bg-elevated)", color: selected === m.name ? "#fff" : "var(--text-primary)" }} onClick={() => setSelected(m.name)}>
            {m.name}
          </button>
        ))}
        <button style={{ ...btnStyle, marginLeft: "auto" }} onClick={() => setShowCompare(!showCompare)}>
          {showCompare ? "Hide" : "Show"} Comparison
        </button>
      </div>

      {/* Comparison matrix */}
      {showCompare && (
        <div style={{ overflowX: "auto", marginBottom: 16 }}>
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
            <thead>
              <tr style={{ borderBottom: "2px solid var(--border-color)" }}>
                <th style={{ textAlign: "left", padding: 6, color: "var(--text-secondary)" }}>Aspect</th>
                {METHODOLOGIES.map(m => <th key={m.name} style={{ textAlign: "center", padding: 6, color: m.name === selected ? "var(--accent-blue)" : "var(--text-secondary)" }}>{m.name}</th>)}
              </tr>
            </thead>
            <tbody>
              {[
                { label: "Iterations", values: ["1-4 week sprints", "Continuous", "1-3 week iterations", "Continuous", "2 week features", "Varies by family", "8-12 week PIs"] },
                { label: "Team Size", values: ["5-9", "Any", "2-12", "Any", "20+", "2-50", "50+"] },
                { label: "Roles", values: ["3 defined", "None required", "4 defined", "None required", "6 defined", "4 defined", "7+ defined"] },
                { label: "Ceremonies", values: ["5", "4", "4", "3", "5", "3", "6+"] },
              ].map(row => (
                <tr key={row.label} style={{ borderBottom: "1px solid var(--border-color)" }}>
                  <td style={{ padding: 6, fontWeight: 500, color: "var(--text-primary)" }}>{row.label}</td>
                  {row.values.map((v, i) => <td key={i} style={{ textAlign: "center", padding: 6, color: "var(--text-secondary)" }}>{v}</td>)}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {/* Selected methodology details */}
      <div style={{ ...cardBaseStyle }}>
        <h3 style={{ margin: "0 0 8px", fontSize: 18, color: "var(--text-primary)" }}>{info.name}</h3>
        <p style={{ fontSize: 13, color: "var(--text-secondary)", lineHeight: 1.5, marginBottom: 12 }}>{info.description}</p>

        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
          <div>
            <div style={sectionTitle}>Core Principles</div>
            <ul style={{ margin: 0, paddingLeft: 16, fontSize: 12, color: "var(--text-primary)" }}>
              {info.principles.map(p => <li key={p} style={{ marginBottom: 4 }}>{p}</li>)}
            </ul>
          </div>
          <div>
            <div style={sectionTitle}>Roles</div>
            <ul style={{ margin: 0, paddingLeft: 16, fontSize: 12, color: "var(--text-primary)" }}>
              {info.roles.map(r => <li key={r} style={{ marginBottom: 4 }}>{r}</li>)}
            </ul>
          </div>
          <div>
            <div style={sectionTitle}>Ceremonies / Practices</div>
            {info.ceremonies.map(c => (
              <div key={c} style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4, cursor: "pointer" }} onClick={() => togglePractice(`${info.name}:${c}`)}>
                <span style={{ fontSize: 14, color: enabledPractices[`${info.name}:${c}`] !== false ? "var(--success-color)" : "var(--text-secondary)" }}>
                  {enabledPractices[`${info.name}:${c}`] !== false ? "[x]" : "[ ]"}
                </span>
                <span style={{ fontSize: 12, color: "var(--text-primary)" }}>{c}</span>
              </div>
            ))}
          </div>
          <div>
            <div style={sectionTitle}>Artifacts</div>
            <ul style={{ margin: 0, paddingLeft: 16, fontSize: 12, color: "var(--text-primary)" }}>
              {info.artifacts.map(a => <li key={a} style={{ marginBottom: 4 }}>{a}</li>)}
            </ul>
          </div>
        </div>

        <div style={{ marginTop: 12, padding: "8px 12px", background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)" }}>
          <span style={{ fontSize: 12, fontWeight: 600, color: "var(--accent-gold)" }}>Best Suited For: </span>
          <span style={{ fontSize: 12, color: "var(--text-secondary)" }}>{info.bestFor}</span>
        </div>
      </div>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════════════════
   AI Coach Tab
   ═══════════════════════════════════════════════════════════════════════ */

function AiCoachTab() {
  const [sprintId, setSprintId] = useState("");
  const [analysis, setAnalysis] = useState<AiAnalysis | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const cancelRef = useRef(false);
  const taskIdRef = useRef<string | null>(null);

  const analyzesprint = useCallback(async () => {
    if (!sprintId.trim()) {
      setError("Enter a Sprint ID to analyze");
      return;
    }
    setLoading(true);
    setError("");
    setAnalysis(null);
    cancelRef.current = false;

    try {
      const result = await invoke<AiAnalysis>("agile_ai_analyze", { sprintId: sprintId.trim() });
      if (cancelRef.current) return;
      taskIdRef.current = result.taskId;
      setAnalysis(result);
    } catch (e: any) {
      if (!cancelRef.current) {
        setError(typeof e === "string" ? e : e?.message || "AI analysis failed");
      }
    } finally {
      if (!cancelRef.current) {
        setLoading(false);
      }
    }
  }, [sprintId]);

  const handleSuspend = useCallback(() => {
    cancelRef.current = true;
    taskIdRef.current = null;
    setLoading(false);
    setError("Analysis suspended by user");
  }, []);

  return (
    <div>
      {error && <div style={{ color: "var(--error-color)", marginBottom: 8, fontSize: 12 }}>{error}</div>}

      <div style={{ display: "flex", gap: 8, marginBottom: 16, alignItems: "center" }}>
        <input style={{ ...inputStyle, width: 200 }} placeholder="Sprint ID" value={sprintId} onChange={e => setSprintId(e.target.value)} />
        <button style={btnPrimaryStyle} onClick={analyzesprint} disabled={loading}>
          {loading ? "Analyzing..." : "Analyze Sprint"}
        </button>
        {loading && (
          <button style={{ ...btnStyle, background: "var(--accent-rose)", color: "#fff", borderColor: "var(--accent-rose)" }} onClick={handleSuspend}>
            Suspend
          </button>
        )}
      </div>

      {loading && (
        <div style={{ textAlign: "center", padding: 24, color: "var(--text-secondary)" }}>
          <div style={{ fontSize: 13, marginBottom: 4 }}>Running AI analysis on sprint data...</div>
          <div style={{ fontSize: 12 }}>This may take a moment. You can suspend at any time.</div>
        </div>
      )}

      {analysis && (
        <div>
          {/* Recommendations */}
          <div style={sectionTitle}>Recommendations</div>
          {analysis.recommendations.map((rec, i) => (
            <div key={i} style={{ ...cardBaseStyle, borderLeft: `3px solid ${riskColor(rec.risk)}` }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                <span style={{ fontWeight: 600, fontSize: 13, color: "var(--text-primary)" }}>{rec.title}</span>
                <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
                  <span style={badgeStyle("var(--bg-tertiary)", "var(--text-secondary)")}>{rec.category}</span>
                  <span style={{ width: 10, height: 10, borderRadius: "50%", background: riskColor(rec.risk), display: "inline-block" }} title={`Risk: ${rec.risk}`} />
                </div>
              </div>
              <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 6 }}>{rec.description}</div>
              {rec.actionItems.length > 0 && (
                <div>
                  <div style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)", marginBottom: 2 }}>Action Items:</div>
                  <ul style={{ margin: 0, paddingLeft: 16, fontSize: 12, color: "var(--text-primary)" }}>
                    {rec.actionItems.map((a, j) => <li key={j} style={{ marginBottom: 2 }}>{a}</li>)}
                  </ul>
                </div>
              )}
            </div>
          ))}

          {/* Bottlenecks */}
          {analysis.bottlenecks.length > 0 && (
            <div style={{ marginTop: 16 }}>
              <div style={sectionTitle}>Bottleneck Detection</div>
              {analysis.bottlenecks.map((b, i) => (
                <div key={i} style={{ ...cardBaseStyle, borderLeft: "3px solid var(--warning-color)", padding: "8px 12px", fontSize: 12, color: "var(--text-primary)" }}>{b}</div>
              ))}
            </div>
          )}

          {/* Story sizing */}
          {analysis.sizingSuggestions.length > 0 && (
            <div style={{ marginTop: 16 }}>
              <div style={sectionTitle}>Story Sizing Suggestions</div>
              {analysis.sizingSuggestions.map((s, i) => (
                <div key={i} style={{ ...cardBaseStyle, borderLeft: "3px solid var(--accent-blue)", padding: "8px 12px", fontSize: 12, color: "var(--text-primary)" }}>{s}</div>
              ))}
            </div>
          )}

          {/* Retro insights */}
          {analysis.retroInsights.length > 0 && (
            <div style={{ marginTop: 16 }}>
              <div style={sectionTitle}>Retrospective Insights</div>
              {analysis.retroInsights.map((r, i) => (
                <div key={i} style={{ ...cardBaseStyle, borderLeft: "3px solid var(--accent-purple)", padding: "8px 12px", fontSize: 12, color: "var(--text-primary)" }}>{r}</div>
              ))}
            </div>
          )}

          {/* Risk assessment */}
          <div style={{ marginTop: 16 }}>
            <div style={sectionTitle}>Risk Assessment</div>
            <div style={{ display: "flex", gap: 12 }}>
              {(["red", "amber", "green"] as RiskLevel[]).map(level => {
                const count = analysis.recommendations.filter(r => r.risk === level).length;
                return (
                  <div key={level} style={{ ...cardBaseStyle, flex: 1, textAlign: "center" }}>
                    <div style={{ width: 24, height: 24, borderRadius: "50%", background: riskColor(level), margin: "0 auto 6px" }} />
                    <div style={{ fontSize: 18, fontWeight: 700, color: "var(--text-primary)" }}>{count}</div>
                    <div style={{ fontSize: 11, color: "var(--text-secondary)", textTransform: "capitalize" }}>{level}</div>
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      )}

      {/* Placeholder when no analysis */}
      {!analysis && !loading && (
        <div style={{ textAlign: "center", padding: 32, color: "var(--text-secondary)" }}>
          <div style={{ fontSize: 14, marginBottom: 4 }}>AI Agile Coach</div>
          <div style={{ fontSize: 12 }}>Enter a Sprint ID and click "Analyze Sprint" to get AI-powered coaching recommendations, bottleneck detection, and process improvement suggestions.</div>
        </div>
      )}
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════════════════
   Main AgilePanel Component
   ═══════════════════════════════════════════════════════════════════════ */

const TABS: { key: TabKey; label: string }[] = [
  { key: "board", label: "Board" },
  { key: "sprint", label: "Sprint" },
  { key: "backlog", label: "Backlog" },
  { key: "ceremonies", label: "Ceremonies" },
  { key: "metrics", label: "Metrics" },
  { key: "methodology", label: "Methodology" },
  { key: "coach", label: "AI Coach" },
];

function AgilePanel() {
  const [activeTab, setActiveTab] = useState<TabKey>("board");

  return (
    <div style={{ padding: 16, height: "100%", overflowY: "auto", background: "var(--bg-primary)", color: "var(--text-primary)" }}>
      <h2 style={{ margin: "0 0 12px", fontSize: 18, fontWeight: 700, background: "var(--gradient-accent)", WebkitBackgroundClip: "text", WebkitTextFillColor: "transparent" }}>
        Agile Project Management
      </h2>

      {/* Tab bar */}
      <div style={tabBarStyle}>
        {TABS.map(t => (
          <button key={t.key} style={tabStyle(activeTab === t.key)} onClick={() => setActiveTab(t.key)}>
            {t.label}
          </button>
        ))}
      </div>

      {/* Tab content */}
      {activeTab === "board" && <BoardTab />}
      {activeTab === "sprint" && <SprintTab />}
      {activeTab === "backlog" && <BacklogTab />}
      {activeTab === "ceremonies" && <CeremoniesTab />}
      {activeTab === "metrics" && <MetricsTab />}
      {activeTab === "methodology" && <MethodologyTab />}
      {activeTab === "coach" && <AiCoachTab />}
    </div>
  );
}

export default AgilePanel;
