/* eslint-disable @typescript-eslint/no-explicit-any */
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
import { X } from "lucide-react";

/* ── Types ───────────────────────────────────────────────────────────── */

type TabKey = "board" | "sprint" | "backlog" | "ceremonies" | "metrics" | "methodology" | "safe" | "coach";
type Priority = "P0" | "P1" | "P2" | "P3";
type Column = "Backlog" | "To Do" | "In Progress" | "In Review" | "Done";
type SprintStatus = "Planning" | "Active" | "Completed" | "Cancelled";
type Methodology = "Scrum" | "Kanban" | "XP" | "Lean" | "FDD" | "Crystal" | "SAFe";
type RiskLevel = "red" | "amber" | "green";

const COLUMNS: Column[] = ["Backlog", "To Do", "In Progress", "In Review", "Done"];
const PRIORITIES: Priority[] = ["P0", "P1", "P2", "P3"];

type SwimlaneMode = "none" | "assignee" | "priority" | "epic";
type BoardMode = "kanban" | "sprint";

interface Subtask {
  id: string;
  title: string;
  done: boolean;
}

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
  epic?: string;
  subtasks?: Subtask[];
  sprintId?: string;
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

type PIStatus = "Planning" | "Executing" | "IP" | "Completed";
type EpicStatus = "Funnel" | "Analyzing" | "Backlog" | "Implementing" | "Done";

interface Feature {
  id: string;
  title: string;
  description: string;
  teamId: string;
  iteration: number;
  businessValue: number;
  timeCriticality: number;
  riskReduction: number;
  jobSize: number;
  status: Column;
}

interface AgileReleaseTrainTeam {
  id: string;
  name: string;
  capacity: number;
  members: number;
  features: string[];
}

interface ProgramIncrement {
  id: string;
  name: string;
  startDate: string;
  endDate: string;
  status: PIStatus;
  iterations: number;
  ipIteration: boolean;
  objectives: PIObjective[];
  features: Feature[];
}

interface PIObjective {
  id: string;
  teamId: string;
  description: string;
  businessValue: number;
  committed: boolean;
  achieved: boolean;
}

interface Epic {
  id: string;
  title: string;
  description: string;
  status: EpicStatus;
  leanBusinessCase: string;
  wsjfScore: number;
  features: string[];
}

interface SAFeData {
  programIncrements: ProgramIncrement[];
  teams: AgileReleaseTrainTeam[];
  epics: Epic[];
}

/* ── Priority colors ────────────────────────────────────────────────── */

const PRIORITY_COLORS: Record<Priority, string> = {
  P0: "var(--error-color)",
  P1: "var(--warning-color)",
  P2: "var(--info-color)",
  P3: "var(--text-secondary)",
};

const riskColor = (r: RiskLevel) =>
  r === "red" ? "var(--error-color)" : r === "amber" ? "var(--warning-color)" : "var(--success-color)";

/* ── Shared styles ──────────────────────────────────────────────────── */

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

const badgeStyle = (bg: string, fg = "white"): React.CSSProperties => ({
  display: "inline-block",
  padding: "2px 8px",
  borderRadius: "var(--radius-sm)",
  fontSize: "var(--font-size-sm)",
  fontWeight: 600,
  background: bg,
  color: fg,
  marginRight: 4,
});




const sectionTitle: React.CSSProperties = {
  fontSize: "var(--font-size-xl)",
  fontWeight: 600,
  color: "var(--text-primary)",
  marginBottom: 10,
};

/* ── StoryDetailModal — Jira-like expandable story editor ─────────────── */

interface StoryDetailModalProps {
  story: Card;
  onSave: (updated: Card) => void;
  onDelete?: () => void;
  onClose: () => void;
  title?: string; // override for AI suggestions ("Review Story")
}

function StoryDetailModal({ story, onSave, onDelete, onClose, title }: StoryDetailModalProps) {
  const [draft, setDraft] = useState<Card>({ ...story });
  const [acInput, setAcInput] = useState("");
  const [labelInput, setLabelInput] = useState(story.labels.join(", "));

  const set = (field: keyof Card, value: unknown) =>
    setDraft(prev => ({ ...prev, [field]: value }));

  const addAC = () => {
    const t = acInput.trim();
    if (!t) return;
    setDraft(prev => ({ ...prev, acceptanceCriteria: [...prev.acceptanceCriteria, t] }));
    setAcInput("");
  };

  const removeAC = (i: number) =>
    setDraft(prev => ({ ...prev, acceptanceCriteria: prev.acceptanceCriteria.filter((_, idx) => idx !== i) }));

  const updateAC = (i: number, val: string) =>
    setDraft(prev => {
      const next = [...prev.acceptanceCriteria];
      next[i] = val;
      return { ...prev, acceptanceCriteria: next };
    });

  const handleSave = () => {
    const labels = labelInput.split(",").map(s => s.trim()).filter(Boolean);
    onSave({ ...draft, labels });
  };

  // Detect if title follows "As a ... I want ... so that ..." format
  const titleLower = draft.title.toLowerCase();
  const followsFormat = titleLower.startsWith("as a") && titleLower.includes("i want") && titleLower.includes("so that");

  // Parse the three parts for display when format is valid
  const parsedStory = (() => {
    if (!followsFormat) return null;
    const t = draft.title;
    const iWant = t.toLowerCase().indexOf("i want");
    const soThat = t.toLowerCase().indexOf("so that");
    return {
      who: t.slice(0, iWant).replace(/^[Aa]s [Aa] /, "").trim().replace(/,$/, ""),
      action: t.slice(iWant + 6, soThat).trim().replace(/,$/, ""),
      benefit: t.slice(soThat + 7).trim(),
    };
  })();

  // Close on Escape
  React.useEffect(() => {
    const handler = (e: KeyboardEvent) => { if (e.key === "Escape") onClose(); };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [onClose]);

  return (
    <div role="button" tabIndex={0}
      style={{
        position: "fixed", inset: 0, zIndex: 9999,
        background: "rgba(0,0,0,0.55)", backdropFilter: "blur(2px)",
        display: "flex", alignItems: "center", justifyContent: "center",
        padding: 24,
      }}
      onClick={onClose}
    >
      <div role="button" tabIndex={0}
        style={{
          background: "var(--bg-elevated)", borderRadius: "var(--radius-lg)",
          border: "1px solid var(--border-color)", boxShadow: "var(--elevation-3, 0 8px 32px rgba(0,0,0,0.4))",
          width: "min(720px, 96vw)", maxHeight: "88vh",
          display: "flex", flexDirection: "column", overflow: "hidden",
        }}
        onClick={e => e.stopPropagation()}
      >
        {/* ── Modal header ── */}
        <div style={{
          display: "flex", alignItems: "center", gap: 10, padding: "16px 20px",
          borderBottom: "1px solid var(--border-color)", flexShrink: 0,
        }}>
          <span style={badgeStyle(PRIORITY_COLORS[draft.priority])}>{draft.priority}</span>
          {draft.epic && <span style={badgeStyle("var(--accent-bg)", "var(--accent-color)")}>{draft.epic}</span>}
          <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", fontFamily: "var(--font-mono)" }}>
            {title ?? "Story"}
          </span>
          <div style={{ flex: 1 }} />
          <button className="panel-btn"
            onClick={onClose}
            style={{ background: "none", border: "none", cursor: "pointer", color: "var(--text-secondary)", lineHeight: 1, padding: "0 4px", display: "flex", alignItems: "center" }}
          ><X size={16} /></button>
        </div>

        {/* ── Scrollable body ── */}
        <div style={{ flex: 1, overflowY: "auto", padding: "20px 20px", display: "flex", flexDirection: "column", gap: 16 }}>

          {/* Title */}
          <div>
            <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
              <label style={{ ...fieldLabel, margin: 0 }}>User Story</label>
              <span style={{
                fontSize: "var(--font-size-xs)", padding: "1px 8px", borderRadius: "var(--radius-md)", fontWeight: 600,
                background: followsFormat ? "color-mix(in srgb, var(--accent-green) 15%, transparent)" : "color-mix(in srgb, var(--accent-gold) 15%, transparent)",
                color: followsFormat ? "var(--text-success)" : "var(--text-warning)",
                border: `1px solid ${followsFormat ? "var(--success-color)" : "var(--warning-color)"}`,
              }}>
                {followsFormat ? "✓ Standard format" : "⚠ Use: As a … I want … so that …"}
              </span>
            </div>
            <input
              className="panel-input panel-input-full"
              style={{ fontSize: "var(--font-size-lg)", fontWeight: 600 }}
              placeholder='As a [type of user], I want [an action] so that [value/benefit]'
              value={draft.title}
              onChange={e => set("title", e.target.value)}
            />
            {/* Parsed story card — shown when format is valid */}
            {parsedStory && (
              <div style={{
                marginTop: 8, padding: "12px 16px", borderRadius: "var(--radius-sm)",
                background: "color-mix(in srgb, var(--accent-blue) 6%, transparent)",
                border: "1px solid color-mix(in srgb, var(--accent-blue) 25%, transparent)",
                display: "flex", flexDirection: "column", gap: 4,
              }}>
                <div style={{ fontSize: "var(--font-size-sm)", display: "flex", gap: 6, flexWrap: "wrap" }}>
                  <span style={{ color: "var(--text-secondary)", fontWeight: 600, minWidth: 50 }}>As a</span>
                  <span style={{ color: "var(--text-primary)", fontWeight: 600 }}>{parsedStory.who}</span>
                </div>
                <div style={{ fontSize: "var(--font-size-sm)", display: "flex", gap: 6, flexWrap: "wrap" }}>
                  <span style={{ color: "var(--text-secondary)", fontWeight: 600, minWidth: 50 }}>I want</span>
                  <span style={{ color: "var(--text-primary)" }}>{parsedStory.action}</span>
                </div>
                <div style={{ fontSize: "var(--font-size-sm)", display: "flex", gap: 6, flexWrap: "wrap" }}>
                  <span style={{ color: "var(--text-secondary)", fontWeight: 600, minWidth: 50 }}>So that</span>
                  <span style={{ color: "var(--accent-color)" }}>{parsedStory.benefit}</span>
                </div>
              </div>
            )}
          </div>

          {/* Description */}
          <div>
            <label style={fieldLabel}>Description</label>
            <textarea
              className="panel-input panel-input-full"
              style={{ minHeight: 80, resize: "vertical", lineHeight: 1.5 }}
              value={draft.description}
              onChange={e => set("description", e.target.value)}
            />
          </div>

          {/* Priority + Points + Column */}
          <div style={{ display: "flex", gap: 12, flexWrap: "wrap" }}>
            <div style={{ flex: 1, minWidth: 120 }}>
              <label style={fieldLabel}>Priority</label>
              <select className="panel-select" style={{ width: "100%" }} value={draft.priority} onChange={e => set("priority", e.target.value as Priority)}>
                {PRIORITIES.map(p => <option key={p} value={p}>{p}</option>)}
              </select>
            </div>
            <div style={{ flex: 1, minWidth: 100 }}>
              <label style={fieldLabel}>Story Points</label>
              <input className="panel-input" style={{ width: "100%" }} type="number" min={0} value={draft.storyPoints} onChange={e => set("storyPoints", Number(e.target.value))} />
            </div>
            <div style={{ flex: 1, minWidth: 140 }}>
              <label style={fieldLabel}>Status</label>
              <select className="panel-select" style={{ width: "100%" }} value={draft.column} onChange={e => set("column", e.target.value as Column)}>
                {COLUMNS.map(c => <option key={c} value={c}>{c}</option>)}
              </select>
            </div>
            <div style={{ flex: 1, minWidth: 140 }}>
              <label style={fieldLabel}>Assignee</label>
              <input className="panel-input" style={{ width: "100%" }} placeholder="Unassigned" value={draft.assignee} onChange={e => set("assignee", e.target.value)} />
            </div>
          </div>

          {/* Epic */}
          <div>
            <label style={fieldLabel}>Epic</label>
            <input
              className="panel-input panel-input-full"
              placeholder="Epic name"
              value={draft.epic ?? ""}
              onChange={e => set("epic", e.target.value)}
            />
          </div>

          {/* Labels */}
          <div>
            <label style={fieldLabel}>Labels <span style={{ fontWeight: 400, opacity: 0.6 }}>(comma-separated)</span></label>
            <input
              className="panel-input panel-input-full"
              value={labelInput}
              onChange={e => setLabelInput(e.target.value)}
              placeholder="backend, api, security"
            />
          </div>

          {/* Acceptance Criteria */}
          <div>
            <label style={fieldLabel}>Acceptance Criteria</label>
            {draft.acceptanceCriteria.map((ac, i) => (
              <div key={i} style={{ display: "flex", gap: 6, marginBottom: 6, alignItems: "flex-start" }}>
                <span style={{ marginTop: 7, fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", fontFamily: "var(--font-mono)", minWidth: 20 }}>{i + 1}.</span>
                <textarea
                  className="panel-input"
                  style={{ flex: 1, minHeight: 36, resize: "vertical", fontSize: "var(--font-size-base)", lineHeight: 1.4 }}
                  value={ac}
                  onChange={e => updateAC(i, e.target.value)}
                />
                <button
                  onClick={() => removeAC(i)}
                  style={{ background: "none", border: "none", cursor: "pointer", color: "var(--text-danger)", padding: "4px", lineHeight: 1, marginTop: 2, display: "flex", alignItems: "center" }}
                ><X size={14} /></button>
              </div>
            ))}
            <div style={{ display: "flex", gap: 6, marginTop: 4 }}>
              <input
                className="panel-input"
                style={{ flex: 1, fontSize: "var(--font-size-base)" }}
                placeholder="Add acceptance criterion..."
                value={acInput}
                onChange={e => setAcInput(e.target.value)}
                onKeyDown={e => { if (e.key === "Enter") { e.preventDefault(); addAC(); } }}
              />
              <button className="panel-btn panel-btn-secondary" onClick={addAC} style={{ fontSize: "var(--font-size-sm)" }}>Add</button>
            </div>
          </div>
        </div>

        {/* ── Footer actions ── */}
        <div style={{
          display: "flex", gap: 8, padding: "12px 20px",
          borderTop: "1px solid var(--border-color)", flexShrink: 0,
        }}>
          <button className="panel-btn panel-btn-primary" onClick={handleSave}>Save</button>
          <button className="panel-btn panel-btn-secondary" onClick={onClose}>Cancel</button>
          <div style={{ flex: 1 }} />
          {onDelete && (
            <button className="panel-btn panel-btn-danger" onClick={onDelete} style={{ fontSize: "var(--font-size-sm)" }}>Delete Story</button>
          )}
        </div>
      </div>
    </div>
  );
}

const fieldLabel: React.CSSProperties = {
  display: "block", fontSize: "var(--font-size-sm)", fontWeight: 600,
  color: "var(--text-secondary)", marginBottom: 4, textTransform: "uppercase", letterSpacing: "0.05em",
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
  epic: undefined,
  subtasks: [],
  sprintId: undefined,
});

/* ═══════════════════════════════════════════════════════════════════════
   Board Tab — Jira-style Kanban / Sprint Board
   ═══════════════════════════════════════════════════════════════════════ */

/* Assignee avatar (colored initials circle) */
const AVATAR_COLORS = ["var(--accent-blue)", "var(--error-color)", "var(--accent-green)", "var(--warning-color)", "var(--accent-purple)", "var(--accent-rose)", "var(--accent-cyan)", "var(--success-color)"];
function AvatarBadge({ name, size = 24 }: { name: string; size?: number }) {
  if (!name) return null;
  const initials = name.split(/\s+/).map(w => w[0]?.toUpperCase() || "").join("").slice(0, 2);
  const colorIdx = name.split("").reduce((a, c) => a + c.charCodeAt(0), 0) % AVATAR_COLORS.length;
  return (
    <div title={name} style={{
      width: size, height: size, borderRadius: "50%", background: AVATAR_COLORS[colorIdx],
      color: "var(--text-primary)", fontSize: size * 0.42, fontWeight: 700, display: "flex", alignItems: "center",
      justifyContent: "center", flexShrink: 0, border: "2px solid var(--bg-primary)",
    }}>
      {initials}
    </div>
  );
}

/* Card age in days */
function cardAgeDays(card: Card): number {
  const created = new Date(card.createdAt).getTime();
  return Math.floor((Date.now() - created) / 86400000);
}
function agingColor(days: number): string | undefined {
  if (days > 14) return "var(--error-color)";
  if (days > 7) return "var(--warning-color)";
  return undefined;
}

/* Subtask progress bar */
function SubtaskProgress({ subtasks }: { subtasks?: Subtask[] }) {
  if (!subtasks || subtasks.length === 0) return null;
  const done = subtasks.filter(s => s.done).length;
  const pct = Math.round((done / subtasks.length) * 100);
  return (
    <div style={{ marginTop: 4 }}>
      <div style={{ display: "flex", justifyContent: "space-between", fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", marginBottom: 2 }}>
        <span>Subtasks</span><span>{done}/{subtasks.length}</span>
      </div>
      <div style={{ height: 4, borderRadius: 2, background: "var(--bg-tertiary)", overflow: "hidden" }}>
        <div style={{ height: "100%", width: `${pct}%`, borderRadius: 2, background: pct === 100 ? "var(--success-color)" : "var(--accent-blue)", transition: "width 0.3s" }} />
      </div>
    </div>
  );
}

function BoardTab({ provider }: { provider?: string } = {}) {
  const [cards, setCards] = useState<Card[]>([]);
  const [wipLimits, setWipLimits] = useState<WipLimits>({ "Backlog": 20, "To Do": 10, "In Progress": 5, "In Review": 5, "Done": 50 });
  const [editingCard, setEditingCard] = useState<Card | null>(null);
  const [addingTo, setAddingTo] = useState<Column | null>(null);
  const [newTitle, setNewTitle] = useState("");
  const [error, setError] = useState("");
  const [hoveredCard, setHoveredCard] = useState<string | null>(null);

  /* Jira-style features */
  const [boardMode, setBoardMode] = useState<BoardMode>("kanban");
  const [swimlane, setSwimlane] = useState<SwimlaneMode>("none");
  const [filterText, setFilterText] = useState("");
  const [filterAssignee, setFilterAssignee] = useState("");
  const [filterPriority, setFilterPriority] = useState<Priority | "">("");
  const [filterLabel, setFilterLabel] = useState("");
  const [dragCardId, setDragCardId] = useState<string | null>(null);
  const [dragOverCol, setDragOverCol] = useState<Column | null>(null);
  const [sprints, setSprints] = useState<Sprint[]>([]);
  const [activeSprint, setActiveSprint] = useState("");

  useEffect(() => {
    (async () => {
      try {
        const data = await invoke<BoardData>("agile_get_board");
        setCards(data.cards || []);
        if (data.wipLimits) setWipLimits(data.wipLimits);
      } catch (_e: any) {
        setError(typeof _e === "string" ? _e : _e?.message || "Failed to load board");
      }
      try {
        const sData = await invoke<{ sprints: Sprint[] }>("agile_get_sprints");
        setSprints(sData.sprints || []);
        const active = (sData.sprints || []).find(s => s.status === "Active");
        if (active) setActiveSprint(active.id);
      } catch { /* no sprints yet */ }
    })();
  }, []);

  /* ── Filtering ── */
  const filteredCards = cards.filter(c => {
    if (boardMode === "sprint" && activeSprint && c.sprintId !== activeSprint) return false;
    if (filterText && !c.title.toLowerCase().includes(filterText.toLowerCase()) && !c.description.toLowerCase().includes(filterText.toLowerCase())) return false;
    if (filterAssignee && c.assignee !== filterAssignee) return false;
    if (filterPriority && c.priority !== filterPriority) return false;
    if (filterLabel && !c.labels.includes(filterLabel)) return false;
    return true;
  });

  const allAssignees = [...new Set(cards.map(c => c.assignee).filter(Boolean))].sort();
  const allLabels = [...new Set(cards.flatMap(c => c.labels))].sort();
  const allEpics = [...new Set(cards.map(c => c.epic).filter(Boolean) as string[])].sort();

  /* ── Swimlane grouping ── */
  const getSwimlanes = (): { key: string; label: string; cards: Card[] }[] => {
    if (swimlane === "none") return [{ key: "all", label: "", cards: filteredCards }];
    if (swimlane === "assignee") {
      const groups = new Map<string, Card[]>();
      filteredCards.forEach(c => { const k = c.assignee || "Unassigned"; groups.set(k, [...(groups.get(k) || []), c]); });
      return [...groups.entries()].map(([k, v]) => ({ key: k, label: k, cards: v }));
    }
    if (swimlane === "priority") {
      return PRIORITIES.map(p => ({ key: p, label: p, cards: filteredCards.filter(c => c.priority === p) })).filter(g => g.cards.length > 0);
    }
    // epic
    const groups = new Map<string, Card[]>();
    filteredCards.forEach(c => { const k = c.epic || "No Epic"; groups.set(k, [...(groups.get(k) || []), c]); });
    return [...groups.entries()].map(([k, v]) => ({ key: k, label: k, cards: v }));
  };

  /* ── Drag & Drop ── */
  const onDragStart = (e: React.DragEvent, cardId: string) => {
    setDragCardId(cardId);
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("text/plain", cardId);
  };
  const onDragOver = (e: React.DragEvent, col: Column) => { e.preventDefault(); e.dataTransfer.dropEffect = "move"; setDragOverCol(col); };
  const onDragLeave = () => setDragOverCol(null);
  const onDrop = async (e: React.DragEvent, col: Column) => {
    e.preventDefault(); setDragOverCol(null);
    const cardId = dragCardId || e.dataTransfer.getData("text/plain");
    if (!cardId) return;
    setDragCardId(null);
    await moveCard(cardId, col);
  };
  const onDragEnd = () => { setDragCardId(null); setDragOverCol(null); };

  /* ── CRUD ── */
  const moveCard = useCallback(async (cardId: string, targetCol: Column) => {
    try {
      await invoke("agile_move_card", { cardId, column: targetCol });
      setCards(prev => prev.map(c => c.id === cardId ? { ...c, column: targetCol } : c));
    } catch (_e: any) {
      setError(typeof _e === "string" ? _e : _e?.message || "Failed to move card");
    }
  }, []);

  const saveCard = useCallback(async (card: Card) => {
    try {
      await invoke("agile_update_card", { card });
      setCards(prev => {
        const idx = prev.findIndex(c => c.id === card.id);
        if (idx >= 0) { const next = [...prev]; next[idx] = card; return next; }
        return [...prev, card];
      });
      setEditingCard(null);
    } catch (_e: any) {
      setError(typeof _e === "string" ? _e : _e?.message || "Failed to save card");
    }
  }, []);

  const deleteCard = useCallback(async (cardId: string) => {
    try {
      await invoke("agile_delete_card", { cardId });
      setCards(prev => prev.filter(c => c.id !== cardId));
      setEditingCard(null);
    } catch (_e: any) {
      setError(typeof _e === "string" ? _e : _e?.message || "Failed to delete card");
    }
  }, []);

  const addCard = useCallback(async (col: Column) => {
    if (!newTitle.trim()) return;
    const card: Card = { ...defaultCard(col), title: newTitle.trim(), sprintId: boardMode === "sprint" ? activeSprint : undefined };
    await saveCard(card);
    setNewTitle("");
    setAddingTo(null);
  }, [newTitle, saveCard, boardMode, activeSprint]);

  /* ── Subtask helpers ── */
  const [subtaskLoading, setSubtaskLoading] = useState(false);

  const addSubtask = () => {
    if (!editingCard) return;
    const title = window.prompt("Subtask title:");
    if (!title) return;
    setEditingCard({ ...editingCard, subtasks: [...(editingCard.subtasks || []), { id: genId(), title, done: false }] });
  };

  const aiGenerateSubtasks = async () => {
    if (!editingCard) return;
    setSubtaskLoading(true);
    try {
      const result = await invoke<{ subtasks: { title: string }[] }>("agile_ai_generate_subtasks", { card: editingCard , provider});
      const newSubtasks = (result.subtasks || []).map(s => ({ id: genId(), title: s.title, done: false }));
      setEditingCard({ ...editingCard, subtasks: [...(editingCard.subtasks || []), ...newSubtasks] });
    } catch (_e: any) {
      setError(typeof _e === "string" ? _e : _e?.message || "AI subtask generation failed");
    } finally {
      setSubtaskLoading(false);
    }
  };
  const toggleSubtask = (subId: string) => {
    if (!editingCard) return;
    setEditingCard({ ...editingCard, subtasks: (editingCard.subtasks || []).map(s => s.id === subId ? { ...s, done: !s.done } : s) });
  };
  const removeSubtask = (subId: string) => {
    if (!editingCard) return;
    setEditingCard({ ...editingCard, subtasks: (editingCard.subtasks || []).filter(s => s.id !== subId) });
  };

  const colIdx = (col: Column) => COLUMNS.indexOf(col);
  const lanes = getSwimlanes();
  const hasFilters = !!(filterText || filterAssignee || filterPriority || filterLabel);

  return (
    <div>
      {error && <div style={{ color: "var(--error-color)", marginBottom: 8, fontSize: "var(--font-size-base)" }}>{error}</div>}

      {/* ── Toolbar: Board Mode + Swimlane + Quick Filters ── */}
      <div style={{ display: "flex", flexWrap: "wrap", gap: 8, alignItems: "center", marginBottom: 10, padding: "8px 0", borderBottom: "1px solid var(--border-color)" }}>
        {/* Board mode toggle */}
        <div style={{ display: "flex", borderRadius: "var(--radius-sm)", overflow: "hidden", border: "1px solid var(--border-color)" }}>
          <button className={`panel-btn panel-tab ${boardMode === "kanban" ? "active" : ""}`} style={{ border: "none", borderRadius: 0, fontSize: "var(--font-size-sm)", padding: "4px 12px" }} onClick={() => setBoardMode("kanban")}>Kanban</button>
          <button className={`panel-btn panel-tab ${boardMode === "sprint" ? "active" : ""}`} style={{ border: "none", borderRadius: 0, fontSize: "var(--font-size-sm)", padding: "4px 12px" }} onClick={() => setBoardMode("sprint")}>Sprint Board</button>
        </div>

        {boardMode === "sprint" && (
          <select className="panel-select" style={{ width: "auto", fontSize: "var(--font-size-sm)", padding: "4px 8px" }} value={activeSprint} onChange={e => setActiveSprint(e.target.value)}>
            <option value="">All Sprints</option>
            {sprints.map(s => <option key={s.id} value={s.id}>{s.name} ({s.status})</option>)}
          </select>
        )}

        <div style={{ width: 1, height: 20, background: "var(--border-color)" }} />

        {/* Swimlane selector */}
        <label style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", display: "flex", alignItems: "center", gap: 4 }}>
          Swimlanes:
          <select className="panel-select" style={{ width: "auto", fontSize: "var(--font-size-sm)", padding: "4px 8px" }} value={swimlane} onChange={e => setSwimlane(e.target.value as SwimlaneMode)}>
            <option value="none">None</option>
            <option value="assignee">Assignee</option>
            <option value="priority">Priority</option>
            <option value="epic">Epic</option>
          </select>
        </label>

        <div style={{ width: 1, height: 20, background: "var(--border-color)" }} />

        {/* Quick filters */}
        <input className="panel-input" style={{ width: 140, fontSize: "var(--font-size-sm)", padding: "4px 8px" }} placeholder="Search cards..." value={filterText} onChange={e => setFilterText(e.target.value)} />
        <select className="panel-select" style={{ width: "auto", fontSize: "var(--font-size-sm)", padding: "4px 8px" }} value={filterAssignee} onChange={e => setFilterAssignee(e.target.value)}>
          <option value="">All Assignees</option>
          {allAssignees.map(a => <option key={a} value={a}>{a}</option>)}
        </select>
        <select className="panel-select" style={{ width: "auto", fontSize: "var(--font-size-sm)", padding: "4px 8px" }} value={filterPriority} onChange={e => setFilterPriority(e.target.value as Priority | "")}>
          <option value="">All Priorities</option>
          {PRIORITIES.map(p => <option key={p} value={p}>{p}</option>)}
        </select>
        {allLabels.length > 0 && (
          <select className="panel-select" style={{ width: "auto", fontSize: "var(--font-size-sm)", padding: "4px 8px" }} value={filterLabel} onChange={e => setFilterLabel(e.target.value)}>
            <option value="">All Labels</option>
            {allLabels.map(l => <option key={l} value={l}>{l}</option>)}
          </select>
        )}
        {hasFilters && (
          <button className="panel-btn panel-btn-secondary" style={{ padding: "3px 8px", fontSize: "var(--font-size-sm)", color: "var(--error-color)" }} onClick={() => { setFilterText(""); setFilterAssignee(""); setFilterPriority(""); setFilterLabel(""); }}>Clear Filters</button>
        )}
        <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginLeft: "auto" }}>{filteredCards.length} card{filteredCards.length !== 1 ? "s" : ""}</span>
      </div>

      {/* ── Swimlaned Board ── */}
      {lanes.map(lane => (
        <div key={lane.key}>
          {lane.label && (
            <div style={{ padding: "8px 8px", margin: "8px 0 4px", background: "var(--bg-tertiary)", borderRadius: "var(--radius-sm)", fontWeight: 600, fontSize: "var(--font-size-base)", color: "var(--text-primary)", display: "flex", alignItems: "center", gap: 8 }}>
              {swimlane === "assignee" && <AvatarBadge name={lane.label} size={20} />}
              {swimlane === "priority" && <span style={badgeStyle(PRIORITY_COLORS[lane.label as Priority] || "var(--bg-tertiary)")}>{lane.label}</span>}
              {(swimlane === "epic" || swimlane === "assignee") && <span>{lane.label}</span>}
              <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", fontWeight: 400 }}>({lane.cards.length})</span>
            </div>
          )}
          <div style={{ display: "flex", gap: 12, overflowX: "auto", paddingBottom: 8, marginBottom: swimlane !== "none" ? 12 : 0 }}>
            {COLUMNS.map(col => {
              const colCards = lane.cards.filter(c => c.column === col);
              const overWip = colCards.length > (wipLimits[col] || 50);
              const isDragTarget = dragOverCol === col;
              return (
                <div
                  key={col}
                  onDragOver={e => onDragOver(e, col)} onDragLeave={onDragLeave} onDrop={e => onDrop(e, col)}
                  style={{
                    minWidth: 220, flex: 1, borderRadius: "var(--radius-md)", padding: 10, transition: "background 0.15s, border 0.15s",
                    background: isDragTarget ? "color-mix(in srgb, var(--accent-blue) 8%, transparent)" : "var(--bg-secondary)",
                    border: isDragTarget ? "2px dashed var(--accent-blue)" : overWip ? "2px solid var(--warning-color)" : "1px solid var(--border-color)",
                  }}
                >
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
                    <span style={{ fontWeight: 600, fontSize: "var(--font-size-md)", color: "var(--text-primary)" }}>{col}</span>
                    <span style={{ fontSize: "var(--font-size-sm)", color: overWip ? "var(--warning-color)" : "var(--text-secondary)" }}>{colCards.length}/{wipLimits[col] || "~"}</span>
                  </div>
                  {overWip && <div style={{ fontSize: "var(--font-size-sm)", color: "var(--warning-color)", marginBottom: 6 }}>WIP limit exceeded!</div>}

                  {colCards.map(card => {
                    const age = cardAgeDays(card);
                    const aging = agingColor(age);
                    const isDragging = dragCardId === card.id;
                    return (
                      <div role="button" tabIndex={0}
                        key={card.id} draggable
                        onDragStart={e => onDragStart(e, card.id)} onDragEnd={onDragEnd}
                        style={{
                          ...cardBaseStyle, cursor: "grab", opacity: isDragging ? 0.4 : 1,
                          transform: hoveredCard === card.id && !isDragging ? "translateY(-2px)" : "none",
                          boxShadow: hoveredCard === card.id ? "var(--elevation-2)" : "var(--card-shadow)",
                          borderLeft: aging ? `3px solid ${aging}` : undefined,
                        }}
                        onMouseEnter={() => setHoveredCard(card.id)} onMouseLeave={() => setHoveredCard(null)}
                        onClick={() => setEditingCard({ ...card })}
                      >
                        {card.epic && <div style={{ fontSize: "var(--font-size-xs)", color: "var(--accent-purple)", fontWeight: 600, marginBottom: 4, textTransform: "uppercase", letterSpacing: 0.5 }}>{card.epic}</div>}
                        <div style={{ fontWeight: 500, fontSize: "var(--font-size-md)", color: "var(--text-primary)", marginBottom: 6 }}>{card.title}</div>
                        <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginBottom: 4, alignItems: "center" }}>
                          <span style={badgeStyle(PRIORITY_COLORS[card.priority])}>{card.priority}</span>
                          {card.storyPoints > 0 && <span style={badgeStyle("var(--accent-purple)")}>{card.storyPoints} pts</span>}
                          {card.labels.map(l => <span key={l} style={badgeStyle("var(--bg-tertiary)", "var(--text-secondary)")}>{l}</span>)}
                        </div>
                        <SubtaskProgress subtasks={card.subtasks} />
                        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginTop: 6 }}>
                          <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                            <AvatarBadge name={card.assignee} size={22} />
                            {card.assignee && <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>{card.assignee}</span>}
                          </div>
                          {age > 3 && <span style={{ fontSize: "var(--font-size-xs)", color: aging || "var(--text-secondary)" }} title="Card age">{age}d</span>}
                        </div>
                        <div role="button" tabIndex={0} style={{ display: "flex", gap: 4, marginTop: 6 }} onClick={e => e.stopPropagation()}>
                          {colIdx(col) > 0 && <button className="panel-btn panel-btn-secondary" style={{ padding: "2px 8px", fontSize: "var(--font-size-sm)" }} onClick={() => moveCard(card.id, COLUMNS[colIdx(col) - 1])}>&larr;</button>}
                          {colIdx(col) < COLUMNS.length - 1 && <button className="panel-btn panel-btn-secondary" style={{ padding: "2px 8px", fontSize: "var(--font-size-sm)" }} onClick={() => moveCard(card.id, COLUMNS[colIdx(col) + 1])}>&rarr;</button>}
                        </div>
                      </div>
                    );
                  })}

                  {lane.key === lanes[0].key && (
                    addingTo === col ? (
                      <div style={{ marginTop: 6 }}>
                        <input className="panel-input panel-input-full" placeholder="Card title..." value={newTitle} onChange={e => setNewTitle(e.target.value)} onKeyDown={e => e.key === "Enter" && addCard(col)} autoFocus />
                        <div style={{ display: "flex", gap: 4, marginTop: 4 }}>
                          <button className="panel-btn panel-btn-primary" onClick={() => addCard(col)}>Add</button>
                          <button className="panel-btn panel-btn-secondary" onClick={() => { setAddingTo(null); setNewTitle(""); }}>Cancel</button>
                        </div>
                      </div>
                    ) : (
                      <button className="panel-btn panel-btn-secondary" style={{ width: "100%", marginTop: 6, fontSize: "var(--font-size-base)" }} onClick={() => setAddingTo(col)}>+ Add Card</button>
                    )
                  )}
                </div>
              );
            })}
          </div>
        </div>
      ))}

      {/* ── Card Edit Modal (Jira-style detail) ── */}
      {editingCard && (
        <div role="button" tabIndex={0} style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.5)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 999 }} onClick={() => setEditingCard(null)}>
          <div role="button" tabIndex={0} style={{ background: "var(--bg-primary)", borderRadius: "var(--radius-md)", padding: 24, width: 540, maxHeight: "85vh", overflowY: "auto", border: "1px solid var(--border-color)", boxShadow: "var(--elevation-2)" }} onClick={e => e.stopPropagation()}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 16 }}>
              <h3 style={{ margin: 0, color: "var(--text-primary)", fontSize: 16 }}>Edit Card</h3>
              <div style={{ display: "flex", gap: 6 }}>
                <button className="panel-btn panel-btn-danger" style={{ padding: "4px 12px", fontSize: "var(--font-size-sm)" }} onClick={() => { if (confirm("Delete this card?")) deleteCard(editingCard.id); }}>Delete</button>
                <button className="panel-btn panel-btn-secondary" style={{ padding: "4px 12px", fontSize: "var(--font-size-sm)" }} onClick={() => setEditingCard(null)}>Close</button>
              </div>
            </div>
            <label style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>Title</label>
            <input className="panel-input panel-input-full" style={{ marginBottom: 8 }} value={editingCard.title} onChange={e => setEditingCard({ ...editingCard, title: e.target.value })} />
            <label style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>Description</label>
            <textarea className="panel-input panel-input-full" style={{ minHeight: 60, marginBottom: 8, resize: "vertical" }} value={editingCard.description} onChange={e => setEditingCard({ ...editingCard, description: e.target.value })} />
            <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
              <div style={{ flex: 1 }}>
                <label style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>Assignee</label>
                <input className="panel-input panel-input-full" value={editingCard.assignee} onChange={e => setEditingCard({ ...editingCard, assignee: e.target.value })} />
              </div>
              <div style={{ flex: 1 }}>
                <label style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>Priority</label>
                <select className="panel-select" value={editingCard.priority} onChange={e => setEditingCard({ ...editingCard, priority: e.target.value as Priority })}>
                  {PRIORITIES.map(p => <option key={p} value={p}>{p}</option>)}
                </select>
              </div>
              <div style={{ width: 80 }}>
                <label style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>Points</label>
                <input className="panel-input panel-input-full" type="number" min={0} value={editingCard.storyPoints} onChange={e => setEditingCard({ ...editingCard, storyPoints: Number(e.target.value) })} />
              </div>
            </div>
            <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
              <div style={{ flex: 1 }}>
                <label style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>Epic</label>
                <input className="panel-input panel-input-full" placeholder="Epic name" value={editingCard.epic || ""} onChange={e => setEditingCard({ ...editingCard, epic: e.target.value || undefined })} list="epic-suggestions" />
                <datalist id="epic-suggestions">{allEpics.map(ep => <option key={ep} value={ep} />)}</datalist>
              </div>
              <div style={{ flex: 1 }}>
                <label style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>Column</label>
                <select className="panel-select" value={editingCard.column} onChange={e => setEditingCard({ ...editingCard, column: e.target.value as Column })}>
                  {COLUMNS.map(c => <option key={c} value={c}>{c}</option>)}
                </select>
              </div>
            </div>
            <label style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>Labels (comma-separated)</label>
            <input className="panel-input panel-input-full" style={{ marginBottom: 8 }} value={editingCard.labels.join(", ")} onChange={e => setEditingCard({ ...editingCard, labels: e.target.value.split(",").map(s => s.trim()).filter(Boolean) })} />

            {/* Acceptance Criteria */}
            <div style={{ marginBottom: 12 }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 4 }}>
                <label style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>Acceptance Criteria</label>
                <button className="panel-btn panel-btn-secondary" style={{ padding: "2px 8px", fontSize: "var(--font-size-sm)", color: "var(--accent-blue)" }} onClick={async () => {
                  try {
                    const result = await invoke<{ criteria: string[] }>("agile_ai_generate_ac", { title: editingCard.title, description: editingCard.description , provider});
                    if (result.criteria?.length) {
                      setEditingCard({ ...editingCard, acceptanceCriteria: [...editingCard.acceptanceCriteria, ...result.criteria] });
                    }
                  } catch (_e: any) {
                    setError(typeof _e === "string" ? _e : "Failed to generate AC");
                  }
                }}>AI Generate</button>
              </div>
              {editingCard.acceptanceCriteria.map((ac, i) => (
                <div key={i} style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4 }}>
                  <span style={{ flex: 1, fontSize: "var(--font-size-base)", color: "var(--text-primary)" }}>{ac}</span>
                  <button className="panel-btn" style={{ background: "none", border: "none", cursor: "pointer", color: "var(--text-secondary)", fontSize: "var(--font-size-base)" }} onClick={() => {
                    setEditingCard({ ...editingCard, acceptanceCriteria: editingCard.acceptanceCriteria.filter((_, j) => j !== i) });
                  }}>x</button>
                </div>
              ))}
              <input className="panel-input panel-input-full" style={{ fontSize: "var(--font-size-base)" }} placeholder="Add acceptance criterion..." onKeyDown={e => {
                if (e.key === "Enter" && (e.target as HTMLInputElement).value.trim()) {
                  setEditingCard({ ...editingCard, acceptanceCriteria: [...editingCard.acceptanceCriteria, (e.target as HTMLInputElement).value.trim()] });
                  (e.target as HTMLInputElement).value = "";
                }
              }} />
            </div>

            {/* Subtasks section */}
            <div style={{ marginBottom: 12, borderTop: "1px solid var(--border-color)", paddingTop: 12 }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
                <label style={{ fontSize: "var(--font-size-md)", fontWeight: 600, color: "var(--text-primary)" }}>Subtasks ({(editingCard.subtasks || []).length})</label>
                <div style={{ display: "flex", gap: 4 }}>
                  <button className="panel-btn panel-btn-secondary" style={{ padding: "3px 12px", fontSize: "var(--font-size-sm)" }} onClick={addSubtask}>+ Add</button>
                  <button className="panel-btn panel-btn-primary" style={{ padding: "3px 12px", fontSize: "var(--font-size-sm)" }} onClick={aiGenerateSubtasks} disabled={subtaskLoading}>
                    {subtaskLoading ? "Generating..." : "AI Generate"}
                  </button>
                </div>
              </div>
              {(editingCard.subtasks || []).map(sub => (
                <div key={sub.id} style={{ display: "flex", alignItems: "center", gap: 8, padding: "4px 0", borderBottom: "1px solid var(--border-color)" }}>
                  <input type="checkbox" checked={sub.done} onChange={() => toggleSubtask(sub.id)} />
                  <span style={{ flex: 1, fontSize: "var(--font-size-base)", textDecoration: sub.done ? "line-through" : "none", color: sub.done ? "var(--text-secondary)" : "var(--text-primary)" }}>{sub.title}</span>
                  <button className="panel-btn" style={{ background: "none", border: "none", cursor: "pointer", color: "var(--text-secondary)", fontSize: "var(--font-size-base)" }} onClick={() => removeSubtask(sub.id)}>x</button>
                </div>
              ))}
              {(editingCard.subtasks || []).length > 0 && <SubtaskProgress subtasks={editingCard.subtasks} />}
            </div>

            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 12 }}>
              Created: {new Date(editingCard.createdAt).toLocaleDateString()} · Age: {cardAgeDays(editingCard)} days
              {editingCard.sprintId && <span> · Sprint: {sprints.find(s => s.id === editingCard.sprintId)?.name || editingCard.sprintId}</span>}
            </div>
            <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
              <button className="panel-btn panel-btn-secondary" onClick={() => setEditingCard(null)}>Cancel</button>
              <button className="panel-btn panel-btn-primary" onClick={() => saveCard(editingCard)}>Save</button>
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
  const [availableBacklog, setAvailableBacklog] = useState<Card[]>([]);

  useEffect(() => {
    (async () => {
      try {
        const data = await invoke<{ sprints: Sprint[]; history: SprintHistory[]; backlog: Card[] }>("agile_get_sprints");
        setSprints(data.sprints || []);
        setHistory(data.history || []);
        setAvailableBacklog(data.backlog || []);
        const active = (data.sprints || []).find(s => s.status === "Active");
        if (active) setCurrent(active);
        else if ((data.sprints || []).length > 0) setCurrent(data.sprints[0]);
      } catch (_e: any) {
        setError(typeof _e === "string" ? _e : _e?.message || "Failed to load sprints");
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
    } catch (_e: any) {
      setError(typeof _e === "string" ? _e : _e?.message || "Failed to create sprint");
    }
  }, [newSprint]);

  const updateSprintStatus = useCallback(async (status: SprintStatus) => {
    if (!current) return;
    const updated = { ...current, status };
    try {
      await invoke("agile_update_sprint", { sprint: updated });
      setCurrent(updated);
      setSprints(prev => prev.map(s => s.id === updated.id ? updated : s));
    } catch (_e: any) {
      setError(typeof _e === "string" ? _e : _e?.message || "Failed to update sprint");
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
      {error && <div style={{ color: "var(--error-color)", marginBottom: 8, fontSize: "var(--font-size-base)" }}>{error}</div>}

      {/* Sprint selector */}
      <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 12 }}>
        <select className="panel-select" style={{ width: "auto" }} value={current?.id || ""} onChange={e => { const s = sprints.find(x => x.id === e.target.value); if (s) setCurrent(s); }}>
          <option value="">Select Sprint</option>
          {sprints.map(s => <option key={s.id} value={s.id}>{s.name} ({s.status})</option>)}
        </select>
        <button className="panel-btn panel-btn-primary" onClick={() => setCreating(true)}>+ New Sprint</button>
        {current && current.status === "Planning" && <button className="panel-btn panel-btn-secondary" style={{ background: "var(--accent-green)", color: "var(--btn-primary-fg)" }} onClick={() => updateSprintStatus("Active")}>Start Sprint</button>}
        {current && current.status === "Active" && <button className="panel-btn panel-btn-secondary" style={{ background: "var(--accent-rose)", color: "var(--btn-error-fg)" }} onClick={() => updateSprintStatus("Completed")}>End Sprint</button>}
      </div>

      {/* Create sprint form */}
      {creating && (
        <div style={{ ...cardBaseStyle, marginBottom: 12 }}>
          <h4 style={{ margin: "0 0 8px", color: "var(--text-primary)" }}>New Sprint</h4>
          <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
            <input className="panel-input" style={{ flex: 2 }} placeholder="Sprint name" value={newSprint.name} onChange={e => setNewSprint({ ...newSprint, name: e.target.value })} />
            <input className="panel-input" style={{ flex: 1 }} type="date" value={newSprint.startDate} onChange={e => setNewSprint({ ...newSprint, startDate: e.target.value })} />
            <input className="panel-input" style={{ flex: 1 }} type="date" value={newSprint.endDate} onChange={e => setNewSprint({ ...newSprint, endDate: e.target.value })} />
          </div>
          <input className="panel-input panel-input-full" style={{ marginBottom: 8 }} placeholder="Sprint goal" value={newSprint.goal} onChange={e => setNewSprint({ ...newSprint, goal: e.target.value })} />
          <div style={{ display: "flex", gap: 8 }}>
            <button className="panel-btn panel-btn-primary" onClick={createSprint}>Create</button>
            <button className="panel-btn panel-btn-secondary" onClick={() => setCreating(false)}>Cancel</button>
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
              <div key={label} style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", padding: "8px 16px", border: "1px solid var(--border-color)" }}>
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>{label}</div>
                <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 600, color: "var(--text-primary)" }}>{value}</div>
              </div>
            ))}
          </div>

          {/* Sprint backlog table */}
          <div style={sectionTitle}>Sprint Backlog</div>
          <div style={{ overflowX: "auto" }}>
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "var(--font-size-md)" }}>
              <thead>
                <tr style={{ borderBottom: "1px solid var(--border-color)" }}>
                  {["Story", "Points", "Assignee", "Status", "Priority"].map(h => (
                    <th key={h} style={{ textAlign: "left", padding: "8px 12px", color: "var(--text-secondary)", fontWeight: 500, fontSize: "var(--font-size-base)" }}>{h}</th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {(current.cards || []).map(c => (
                  <tr key={c.id} style={{ borderBottom: "1px solid var(--border-color)" }}>
                    <td style={{ padding: "8px 12px", color: "var(--text-primary)" }}>{c.title}</td>
                    <td style={{ padding: "8px 12px" }}><span style={badgeStyle("var(--accent-purple)")}>{c.storyPoints}</span></td>
                    <td style={{ padding: "8px 12px", color: "var(--text-secondary)" }}>{c.assignee || "-"}</td>
                    <td style={{ padding: "8px 12px" }}><span style={badgeStyle("var(--accent-blue)")}>{c.column}</span></td>
                    <td style={{ padding: "8px 12px" }}><span style={badgeStyle(PRIORITY_COLORS[c.priority])}>{c.priority}</span></td>
                  </tr>
                ))}
                {(current.cards || []).length === 0 && (
                  <tr><td colSpan={5} style={{ padding: 16, textAlign: "center", color: "var(--text-secondary)" }}>No stories in this sprint</td></tr>
                )}
              </tbody>
            </table>
          </div>

          {/* Pull from backlog */}
          {availableBacklog.length > 0 && current && current.status !== "Completed" && (
            <div style={{ marginTop: 12 }}>
              <div style={sectionTitle}>Add from Backlog ({availableBacklog.length} available)</div>
              <div style={{ display: "flex", flexDirection: "column", gap: 4, maxHeight: 200, overflowY: "auto" }}>
                {availableBacklog.map(item => (
                  <div key={item.id} style={{ display: "flex", alignItems: "center", gap: 8, padding: "8px 12px", background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)" }}>
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div style={{ fontSize: "var(--font-size-base)", fontWeight: 500, color: "var(--text-primary)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{item.title}</div>
                      <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>{item.priority} · {item.storyPoints} pts</div>
                    </div>
                    <button
                      className="panel-btn panel-btn-primary"
                      style={{ fontSize: "var(--font-size-sm)", padding: "3px 8px", flexShrink: 0 }}
                      onClick={async () => {
                        const updated: Sprint = {
                          ...current,
                          cards: [...(current.cards || []), { ...item, sprintId: current.id }],
                          plannedPoints: current.plannedPoints + item.storyPoints,
                        };
                        try {
                          await invoke("agile_update_sprint", { sprint: updated });
                          setCurrent(updated);
                          setSprints(prev => prev.map(s => s.id === updated.id ? updated : s));
                          setAvailableBacklog(prev => prev.filter(b => b.id !== item.id));
                        } catch (_e: any) {
                          setError(typeof _e === "string" ? _e : "Failed to add to sprint");
                        }
                      }}
                    >
                      Add to Sprint
                    </button>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Burndown */}
          <div style={{ ...sectionTitle, marginTop: 16 }}>Burndown Chart</div>
          <pre style={{ background: "var(--bg-secondary)", padding: 12, borderRadius: "var(--radius-sm)", fontSize: "var(--font-size-sm)", overflow: "auto", color: "var(--text-primary)", border: "1px solid var(--border-color)" }}>
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
                <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", color: "var(--text-primary)" }}>{h.name}</div>
                <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>Velocity: {h.velocity} | {h.completedPoints}/{h.plannedPoints} pts</div>
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

interface AiSuggestion {
  title: string;
  description: string;
  storyPoints: number;
  priority: Priority;
  labels: string[];
  acceptanceCriteria: string[];
  epic: string;
  dependsOn: number[];
  order: number;
  _accepted?: boolean;
}

function BacklogTab({ provider }: { provider?: string } = {}) {
  const [items, setItems] = useState<Card[]>([]);
  const [filterPriority, setFilterPriority] = useState<Priority | "">("");
  const [filterLabel, setFilterLabel] = useState("");
  const [filterAssignee, setFilterAssignee] = useState("");
  const [showCreate, setShowCreate] = useState(false);
  const [newStory, setNewStory] = useState({ title: "", description: "", storyPoints: 0, priority: "P2" as Priority, labels: "", acceptanceCriteria: "" });
  const [error, setError] = useState("");

  // AI generation state
  const [aiPrompt, setAiPrompt] = useState("");
  const [aiGenerating, setAiGenerating] = useState(false);
  const [aiSuggestions, setAiSuggestions] = useState<AiSuggestion[]>([]);
  const [aiEpics, setAiEpics] = useState<string[]>([]);
  const [showAiGenerate, setShowAiGenerate] = useState(false);

  // Story detail modal
  const [detailCard, setDetailCard] = useState<Card | null>(null);
  const [detailSuggIdx, setDetailSuggIdx] = useState<number | null>(null);

  useEffect(() => {
    (async () => {
      try {
        const data = await invoke<Card[]>("agile_get_backlog");
        setItems(data || []);
      } catch (_e: any) {
        setError(typeof _e === "string" ? _e : _e?.message || "Failed to load backlog");
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
    } catch (_e: any) {
      setError(typeof _e === "string" ? _e : _e?.message || "Failed to create story");
    }
  }, [newStory]);

  const updateInline = useCallback(async (id: string, field: "storyPoints" | "priority", value: any) => {
    const item = items.find(c => c.id === id);
    if (!item) return;
    const updated = { ...item, [field]: value };
    try {
      await invoke("agile_update_story", { story: updated });
      setItems(prev => prev.map(c => c.id === id ? updated : c));
    } catch (_e: any) {
      setError(typeof _e === "string" ? _e : _e?.message || "Failed to update story");
    }
  }, [items]);

  const saveDetailCard = useCallback(async (updated: Card) => {
    try {
      await invoke("agile_update_story", { story: updated });
      setItems(prev => prev.map(c => c.id === updated.id ? updated : c));
    } catch (_e: any) {
      setError(typeof _e === "string" ? _e : _e?.message || "Failed to save story");
    }
    setDetailCard(null);
  }, []);

  const deleteDetailCard = useCallback(async () => {
    if (!detailCard) return;
    try {
      await invoke("agile_delete_story", { storyId: detailCard.id });
      setItems(prev => prev.filter(c => c.id !== detailCard.id));
    } catch (_e: any) {
      setError(typeof _e === "string" ? _e : _e?.message || "Failed to delete story");
    }
    setDetailCard(null);
  }, [detailCard]);

  const saveDetailSuggestion = useCallback((idx: number, updated: Card) => {
    setAiSuggestions(prev => prev.map((s, i) => i !== idx ? s : {
      ...s,
      title: updated.title,
      description: updated.description,
      priority: updated.priority,
      storyPoints: updated.storyPoints,
      labels: updated.labels,
      acceptanceCriteria: updated.acceptanceCriteria,
      epic: updated.epic ?? s.epic,
    }));
    setDetailSuggIdx(null);
  }, []);

  const [splitLoading, setSplitLoading] = useState<string | null>(null);

  const suggestSplit = useCallback(async (id: string) => {
    const item = items.find(c => c.id === id);
    if (!item) return;
    setSplitLoading(id);
    try {
      const result = await invoke<{ stories: { title: string; description: string; storyPoints: number; acceptanceCriteria: string[] }[]; rationale: string }>("agile_ai_split_story", { story: item , provider});
      if (result.stories && result.stories.length > 0) {
        const newCards: Card[] = result.stories.map(s => ({
          ...item,
          id: genId(),
          title: s.title,
          description: s.description || item.description,
          storyPoints: s.storyPoints,
          acceptanceCriteria: s.acceptanceCriteria || [],
          createdAt: new Date().toISOString(),
        }));
        for (const card of newCards) {
          await invoke("agile_create_story", { story: card });
        }
        await invoke("agile_delete_story", { storyId: id });
        setItems(prev => [...newCards, ...prev.filter(c => c.id !== id)]);
      } else {
        setError("AI could not determine a good split for this story.");
        setTimeout(() => setError(""), 3000);
      }
    } catch (_e: any) {
      setError(typeof _e === "string" ? _e : _e?.message || "AI split failed");
    } finally {
      setSplitLoading(null);
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [items]);

  // AI backlog generation
  const handleAiGenerate = useCallback(async () => {
    if (!aiPrompt.trim()) return;
    setAiGenerating(true);
    setAiSuggestions([]);
    setAiEpics([]);
    setError("");
    try {
      const result = await invoke<{ epics?: string[]; stories?: AiSuggestion[] }>("agile_ai_generate_backlog", { prompt: aiPrompt.trim() , provider});
      const stories = (result.stories || []).map(s => ({
        ...s,
        priority: (["P0","P1","P2","P3"].includes(s.priority) ? s.priority : "P2") as Priority,
        labels: s.labels || [],
        acceptanceCriteria: s.acceptanceCriteria || [],
        dependsOn: s.dependsOn || [],
        order: s.order ?? 0,
        _accepted: true,
      }));
      stories.sort((a, b) => a.order - b.order);
      setAiSuggestions(stories);
      setAiEpics(result.epics || []);
    } catch (_e: any) {
      setError(typeof _e === "string" ? _e : _e?.message || "AI generation failed");
    } finally {
      setAiGenerating(false);
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [aiPrompt]);

  const toggleSuggestion = (idx: number) => {
    setAiSuggestions(prev => prev.map((s, i) => i === idx ? { ...s, _accepted: !s._accepted } : s));
  };

  const acceptAllSuggestions = () => setAiSuggestions(prev => prev.map(s => ({ ...s, _accepted: true })));
  const rejectAllSuggestions = () => setAiSuggestions(prev => prev.map(s => ({ ...s, _accepted: false })));

  const commitAccepted = useCallback(async () => {
    const accepted = aiSuggestions.filter(s => s._accepted);
    if (accepted.length === 0) return;
    const newCards: Card[] = accepted.map((s) => ({
      id: genId(),
      title: s.title,
      description: s.description,
      assignee: "",
      priority: s.priority,
      storyPoints: s.storyPoints,
      labels: [...s.labels, ...(s.epic ? [`epic:${s.epic}`] : [])],
      column: "Backlog" as Column,
      acceptanceCriteria: s.acceptanceCriteria,
      createdAt: new Date().toISOString(),
      epic: s.epic,
    }));
    try {
      for (const card of newCards) {
        await invoke("agile_create_story", { story: card });
      }
      setItems(prev => [...newCards, ...prev]);
      setAiSuggestions([]);
      setAiEpics([]);
      setAiPrompt("");
      setShowAiGenerate(false);
    } catch (_e: any) {
      setError(typeof _e === "string" ? _e : _e?.message || "Failed to add stories");
    }
  }, [aiSuggestions]);

  const filtered = items.filter(c => {
    if (filterPriority && c.priority !== filterPriority) return false;
    if (filterLabel && !c.labels.some(l => l.toLowerCase().includes(filterLabel.toLowerCase()))) return false;
    if (filterAssignee && !c.assignee.toLowerCase().includes(filterAssignee.toLowerCase())) return false;
    return true;
  });

  const acceptedCount = aiSuggestions.filter(s => s._accepted).length;

  return (
    <div>
      {error && <div style={{ color: "var(--error-color)", marginBottom: 8, fontSize: "var(--font-size-base)" }}>{error}</div>}

      {/* AI Backlog Generation */}
      <div style={{ marginBottom: 12 }}>
        {showAiGenerate ? (
          <div style={{ ...cardBaseStyle }}>
            <h4 style={{ margin: "0 0 8px", color: "var(--text-primary)" }}>AI Backlog Generator</h4>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 8, lineHeight: 1.5 }}>
              Describe what you want to build. The AI will analyze your project structure and generate epics, stories, estimates, dependencies, and ordering.
            </div>
            <textarea
              className="panel-input panel-input-full" style={{ marginBottom: 8, minHeight: 80, resize: "vertical" }}
              placeholder="e.g., Build a user authentication system with OAuth2, email/password login, role-based access control, and password reset flow..."
              value={aiPrompt}
              onChange={e => setAiPrompt(e.target.value)}
              onKeyDown={e => { if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) handleAiGenerate(); }}
            />
            <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
              <button className="panel-btn panel-btn-primary" onClick={handleAiGenerate} disabled={aiGenerating || !aiPrompt.trim()}>
                {aiGenerating ? "Analyzing project..." : "Generate Backlog"}
              </button>
              <button className="panel-btn panel-btn-secondary" onClick={() => { setShowAiGenerate(false); setAiSuggestions([]); setAiEpics([]); }}>Cancel</button>
              {aiGenerating && <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>AI is scanning files and generating stories...</span>}
            </div>

            {/* Suggestions review */}
            {aiSuggestions.length > 0 && (
              <div style={{ marginTop: 12 }}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
                  <div style={{ fontSize: "var(--font-size-md)", fontWeight: 600, color: "var(--text-primary)" }}>
                    {aiSuggestions.length} stories generated
                    {aiEpics.length > 0 && <span style={{ fontWeight: 400, color: "var(--text-secondary)" }}> across {aiEpics.length} epic{aiEpics.length !== 1 ? "s" : ""}</span>}
                  </div>
                  <div style={{ display: "flex", gap: 6 }}>
                    <button className="panel-btn panel-btn-secondary" style={{ fontSize: "var(--font-size-sm)", padding: "3px 8px" }} onClick={acceptAllSuggestions}>Accept All</button>
                    <button className="panel-btn panel-btn-secondary" style={{ fontSize: "var(--font-size-sm)", padding: "3px 8px" }} onClick={rejectAllSuggestions}>Reject All</button>
                  </div>
                </div>

                {/* Epic legend */}
                {aiEpics.length > 0 && (
                  <div style={{ display: "flex", gap: 6, marginBottom: 8, flexWrap: "wrap" }}>
                    {aiEpics.map(ep => (
                      <span key={ep} style={badgeStyle("var(--accent-bg)", "var(--accent-color)")}>{ep}</span>
                    ))}
                  </div>
                )}

                {/* Story list */}
                {aiSuggestions.map((s, idx) => {
                  const deps = s.dependsOn?.filter(d => d < aiSuggestions.length).map(d => aiSuggestions[d]?.title).filter(Boolean) || [];
                  return (
                    <div
                      key={idx}
                      style={{
                        ...cardBaseStyle,
                        opacity: s._accepted ? 1 : 0.45,
                        borderLeft: `3px solid ${s._accepted ? "var(--accent-color)" : "var(--border-color)"}`,
                      }}
                    >
                      <div style={{ display: "flex", alignItems: "flex-start", gap: 10 }}>
                        {/* Accept checkbox */}
                        <div role="button" tabIndex={0}
                          style={{
                            width: 20, height: 20, borderRadius: "var(--radius-xs-plus)", flexShrink: 0, marginTop: 2, cursor: "pointer",
                            background: s._accepted ? "var(--accent-color)" : "var(--bg-tertiary)",
                            border: `1px solid ${s._accepted ? "var(--accent-color)" : "var(--border-color)"}`,
                            display: "flex", alignItems: "center", justifyContent: "center",
                            color: "var(--btn-primary-fg)", fontSize: "var(--font-size-base)", fontWeight: 700,
                          }}
                          onClick={() => toggleSuggestion(idx)}
                        >
                          {s._accepted ? "\u2713" : ""}
                        </div>
                        {/* Main content — click opens detail */}
                        <div role="button" tabIndex={0}
                          style={{ flex: 1, minWidth: 0, cursor: "pointer" }}
                          onClick={() => setDetailSuggIdx(idx)}
                        >
                          <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 3 }}>
                            <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", fontFamily: "var(--font-mono)" }}>#{idx + 1}</span>
                            <span style={{ fontWeight: 600, fontSize: "var(--font-size-md)", color: "var(--text-primary)" }}>{s.title}</span>
                          </div>
                          <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 4, lineHeight: 1.4 }}>
                            {s.description.length > 120 ? s.description.slice(0, 120) + "…" : s.description}
                          </div>
                          <div style={{ display: "flex", gap: 4, flexWrap: "wrap", alignItems: "center" }}>
                            <span style={badgeStyle("var(--bg-tertiary)", "var(--text-secondary)")}>{s.priority}</span>
                            <span style={badgeStyle("var(--bg-tertiary)", "var(--text-secondary)")}>{s.storyPoints} pts</span>
                            {s.epic && <span style={badgeStyle("var(--accent-bg)", "var(--accent-color)")}>{s.epic}</span>}
                            {s.labels.map(l => <span key={l} style={badgeStyle("var(--bg-tertiary)", "var(--text-secondary)")}>{l}</span>)}
                            {s.acceptanceCriteria.length > 0 && (
                              <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>AC: {s.acceptanceCriteria.length}</span>
                            )}
                            {deps.length > 0 && (
                              <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>
                                ↳ {deps.join(", ")}
                              </span>
                            )}
                          </div>
                        </div>
                        <button
                          className="panel-btn panel-btn-secondary"
                          style={{ fontSize: "var(--font-size-xs)", padding: "2px 8px", flexShrink: 0 }}
                          onClick={() => setDetailSuggIdx(idx)}
                          title="Edit story"
                        >Edit</button>
                      </div>
                    </div>
                  );
                })}

                <div style={{ display: "flex", gap: 8, marginTop: 8 }}>
                  <button className="panel-btn panel-btn-primary" onClick={commitAccepted} disabled={acceptedCount === 0}>
                    Add {acceptedCount} Stor{acceptedCount !== 1 ? "ies" : "y"} to Backlog
                  </button>
                  <button className="panel-btn panel-btn-secondary" onClick={() => { setAiSuggestions([]); setAiEpics([]); }}>Discard All</button>
                </div>
              </div>
            )}
          </div>
        ) : (
          <div style={{ display: "flex", gap: 8 }}>
            <button className="panel-btn panel-btn-primary" onClick={() => setShowAiGenerate(true)}>AI Generate Backlog</button>
            <button className="panel-btn panel-btn-secondary" onClick={() => setShowCreate(true)}>+ Create Story</button>
          </div>
        )}
      </div>

      {/* Manual create form */}
      {showCreate && !showAiGenerate && (
        <div style={{ ...cardBaseStyle, marginBottom: 12 }}>
          <h4 style={{ margin: "0 0 4px", color: "var(--text-primary)" }}>Create Story</h4>
          <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 8, lineHeight: 1.5 }}>
            Use the standard format: <em style={{ color: "var(--accent-color)" }}>"As a [user], I want [action] so that [benefit]"</em>
          </div>
          <input
            className="panel-input panel-input-full"
            style={{ marginBottom: 6 }}
            placeholder='As a [type of user], I want [an action] so that [value/benefit]'
            value={newStory.title}
            onChange={e => setNewStory({ ...newStory, title: e.target.value })}
          />
          <textarea
            className="panel-input panel-input-full"
            style={{ marginBottom: 6, minHeight: 50, resize: "vertical" }}
            placeholder="Describe the user value and business context. Focus on WHY, not HOW."
            value={newStory.description}
            onChange={e => setNewStory({ ...newStory, description: e.target.value })}
          />
          <div style={{ display: "flex", gap: 8, marginBottom: 6 }}>
            <select className="panel-select" style={{ width: "auto" }} value={newStory.priority} onChange={e => setNewStory({ ...newStory, priority: e.target.value as Priority })}>
              {PRIORITIES.map(p => <option key={p} value={p}>{p}</option>)}
            </select>
            <input className="panel-input" style={{ width: 80 }} type="number" min={0} placeholder="Points" value={newStory.storyPoints || ""} onChange={e => setNewStory({ ...newStory, storyPoints: Number(e.target.value) })} />
            <input className="panel-input" style={{ flex: 1 }} placeholder="Labels (comma-separated)" value={newStory.labels} onChange={e => setNewStory({ ...newStory, labels: e.target.value })} />
          </div>
          <textarea className="panel-input panel-input-full" style={{ marginBottom: 8, minHeight: 40, resize: "vertical" }} placeholder="Acceptance criteria (one per line)" value={newStory.acceptanceCriteria} onChange={e => setNewStory({ ...newStory, acceptanceCriteria: e.target.value })} />
          <div style={{ display: "flex", gap: 8 }}>
            <button className="panel-btn panel-btn-primary" onClick={createStory}>Create</button>
            <button className="panel-btn panel-btn-secondary" onClick={() => setShowCreate(false)}>Cancel</button>
          </div>
        </div>
      )}

      {/* Filters */}
      <div style={{ display: "flex", gap: 8, marginBottom: 12, flexWrap: "wrap", alignItems: "center" }}>
        <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>Filter:</span>
        <select className="panel-select" style={{ width: "auto" }} value={filterPriority} onChange={e => setFilterPriority(e.target.value as Priority | "")}>
          <option value="">All Priorities</option>
          {PRIORITIES.map(p => <option key={p} value={p}>{p}</option>)}
        </select>
        <input className="panel-input" style={{ width: 140 }} placeholder="Label" value={filterLabel} onChange={e => setFilterLabel(e.target.value)} />
        <input className="panel-input" style={{ width: 140 }} placeholder="Assignee" value={filterAssignee} onChange={e => setFilterAssignee(e.target.value)} />
        <button className="panel-btn panel-btn-primary" style={{ fontSize: "var(--font-size-sm)", padding: "4px 12px", marginLeft: "auto" }} onClick={async () => {
          const unestimated = items.filter(c => c.storyPoints === 0);
          if (unestimated.length === 0) { setError("All stories already have estimates."); setTimeout(() => setError(""), 3000); return; }
          try {
            const result = await invoke<{ estimates: { id: string; points: number; confidence: string; reasoning: string }[] }>("agile_ai_estimate_points", { stories: unestimated , provider});
            if (result.estimates?.length) {
              const updates = new Map(result.estimates.map(e => [e.id, e]));
              const next = items.map(c => {
                const est = updates.get(c.id);
                return est ? { ...c, storyPoints: est.points } : c;
              });
              setItems(next);
              for (const est of result.estimates) {
                const card = next.find(c => c.id === est.id);
                if (card) await invoke("agile_update_story", { story: card });
              }
            }
          } catch (_e: any) {
            setError(typeof _e === "string" ? _e : "AI estimation failed");
          }
        }}>AI Estimate</button>
      </div>

      {/* Backlog list */}
      {filtered.map(item => (
        <div role="button" tabIndex={0}
          key={item.id}
          style={{ ...cardBaseStyle, cursor: "pointer" }}
          onClick={() => setDetailCard(item)}
        >
          <div style={{ display: "flex", alignItems: "flex-start", gap: 10 }}>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", color: "var(--text-primary)", marginBottom: 2 }}>{item.title}</div>
              {item.description && (
                <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 4, lineHeight: 1.4 }}>
                  {item.description.length > 120 ? item.description.slice(0, 120) + "…" : item.description}
                </div>
              )}
              <div style={{ display: "flex", gap: 4, flexWrap: "wrap", alignItems: "center" }}>
                <span style={badgeStyle(PRIORITY_COLORS[item.priority])}>{item.priority}</span>
                <span style={badgeStyle("var(--accent-purple)")}>{item.storyPoints} pts</span>
                {item.epic && <span style={badgeStyle("var(--accent-bg)", "var(--accent-color)")}>{item.epic}</span>}
                {item.labels.map(l => <span key={l} style={badgeStyle("var(--bg-tertiary)", "var(--text-secondary)")}>{l}</span>)}
                {item.acceptanceCriteria.length > 0 && (
                  <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>AC: {item.acceptanceCriteria.length}</span>
                )}
              </div>
            </div>
            {/* Quick inline controls */}
            <div role="button" tabIndex={0} style={{ display: "flex", gap: 6, alignItems: "center", flexShrink: 0 }} onClick={e => e.stopPropagation()}>
              <select
                className="panel-input" style={{ width: 60, fontSize: "var(--font-size-sm)" }}
                value={item.priority}
                onChange={e => updateInline(item.id, "priority", e.target.value)}
              >
                {PRIORITIES.map(p => <option key={p} value={p}>{p}</option>)}
              </select>
              <input
                className="panel-input" style={{ width: 50, textAlign: "center", fontSize: "var(--font-size-sm)" }}
                type="number" min={0}
                value={item.storyPoints}
                onChange={e => updateInline(item.id, "storyPoints", Number(e.target.value))}
              />
              <button
                className="panel-btn panel-btn-primary"
                style={{ fontSize: "var(--font-size-sm)", padding: "4px 8px" }}
                title="AI-powered story decomposition"
                onClick={() => suggestSplit(item.id)}
                disabled={splitLoading === item.id}
              >
                {splitLoading === item.id ? "…" : "AI Split"}
              </button>
            </div>
          </div>
        </div>
      ))}
      {filtered.length === 0 && !showAiGenerate && (
        <div style={{ textAlign: "center", color: "var(--text-secondary)", padding: 24 }}>
          No backlog items found. Use "AI Generate Backlog" to get started.
        </div>
      )}

      {/* Story detail modal for committed backlog items */}
      {detailCard && (
        <StoryDetailModal
          story={detailCard}
          onSave={saveDetailCard}
          onDelete={deleteDetailCard}
          onClose={() => setDetailCard(null)}
          title={`Story · ${detailCard.column}`}
        />
      )}

      {/* Story detail modal for AI suggestion items */}
      {detailSuggIdx !== null && aiSuggestions[detailSuggIdx] && (() => {
        const s = aiSuggestions[detailSuggIdx];
        const asCard: Card = {
          id: `sugg-${detailSuggIdx}`,
          title: s.title,
          description: s.description,
          priority: s.priority,
          storyPoints: s.storyPoints,
          labels: s.labels,
          acceptanceCriteria: s.acceptanceCriteria,
          epic: s.epic,
          column: "Backlog",
          assignee: "",
          createdAt: "",
        };
        return (
          <StoryDetailModal
            story={asCard}
            onSave={updated => saveDetailSuggestion(detailSuggIdx, updated)}
            onClose={() => setDetailSuggIdx(null)}
            title={`AI Suggestion #${detailSuggIdx + 1}`}
          />
        );
      })()}
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════════════════
   Ceremonies Tab
   ═══════════════════════════════════════════════════════════════════════ */

function CeremoniesTab({ provider }: { provider?: string } = {}) {
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
      } catch (_e: any) {
        setError(typeof _e === "string" ? _e : _e?.message || "Failed to load ceremonies");
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
    } catch (_e: any) {
      setError(typeof _e === "string" ? _e : _e?.message || "Failed to save ceremony");
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
    <button className={`panel-tab ${subTab === key ? "active" : ""}`} onClick={() => setSubTab(key)}>{label}</button>
  );

  return (
    <div>
      {error && <div style={{ color: "var(--error-color)", marginBottom: 8, fontSize: "var(--font-size-base)" }}>{error}</div>}
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
              <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", color: "var(--accent-blue)", marginBottom: 4 }}>{s.member}</div>
              <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-primary)" }}><strong>Did:</strong> {s.didYesterday}</div>
              <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-primary)" }}><strong>Will do:</strong> {s.willDoToday}</div>
              {s.blockers && <div style={{ fontSize: "var(--font-size-base)", color: "var(--error-color)" }}><strong>Blockers:</strong> {s.blockers}</div>}
            </div>
          ))}
          <div style={{ ...cardBaseStyle, background: "var(--bg-secondary)" }}>
            <input className="panel-input panel-input-full" style={{ marginBottom: 4 }} placeholder="Team member" value={newStandup.member} onChange={e => setNewStandup({ ...newStandup, member: e.target.value })} />
            <input className="panel-input panel-input-full" style={{ marginBottom: 4 }} placeholder="What I did yesterday" value={newStandup.didYesterday} onChange={e => setNewStandup({ ...newStandup, didYesterday: e.target.value })} />
            <input className="panel-input panel-input-full" style={{ marginBottom: 4 }} placeholder="What I'll do today" value={newStandup.willDoToday} onChange={e => setNewStandup({ ...newStandup, willDoToday: e.target.value })} />
            <input className="panel-input panel-input-full" style={{ marginBottom: 6 }} placeholder="Blockers (if any)" value={newStandup.blockers} onChange={e => setNewStandup({ ...newStandup, blockers: e.target.value })} />
            <button className="panel-btn panel-btn-primary" onClick={addStandup}>Add Entry</button>
          </div>
        </div>
      )}

      {/* Sprint Planning */}
      {subTab === "planning" && (
        <div>
          <div style={sectionTitle}>Capacity Calculator</div>
          <div style={{ display: "flex", gap: 12, marginBottom: 12, flexWrap: "wrap" }}>
            <div>
              <label style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>Team Members</label>
              <input className="panel-input" style={{ width: 80 }} type="number" min={1} value={capacity.members} onChange={e => { const v = { ...capacity, members: Number(e.target.value) }; setCapacity(v); saveCeremony({ capacity: v }); }} />
            </div>
            <div>
              <label style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>Available Days</label>
              <input className="panel-input" style={{ width: 80 }} type="number" min={1} value={capacity.days} onChange={e => { const v = { ...capacity, days: Number(e.target.value) }; setCapacity(v); saveCeremony({ capacity: v }); }} />
            </div>
            <div>
              <label style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>Focus Factor</label>
              <input className="panel-input" style={{ width: 80 }} type="number" step={0.05} min={0} max={1} value={capacity.focusFactor} onChange={e => { const v = { ...capacity, focusFactor: Number(e.target.value) }; setCapacity(v); saveCeremony({ capacity: v }); }} />
            </div>
          </div>
          <div style={{ ...cardBaseStyle, background: "var(--bg-secondary)" }}>
            <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 600, color: "var(--accent-green)" }}>
              Total Capacity: {(capacity.members * capacity.days * capacity.focusFactor).toFixed(1)} person-days
            </div>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginTop: 4 }}>
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
            <div key={i} role="checkbox" aria-checked={d.done} tabIndex={0} style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6, cursor: "pointer" }} onClick={() => toggleDemo(i)} onKeyDown={e => e.key === "Enter" && toggleDemo(i)}>
              <span style={{ fontSize: 16, color: d.done ? "var(--success-color)" : "var(--text-secondary)" }}>{d.done ? "[x]" : "[ ]"}</span>
              <span style={{ fontSize: "var(--font-size-md)", color: d.done ? "var(--text-secondary)" : "var(--text-primary)", textDecoration: d.done ? "line-through" : "none" }}>{d.item}</span>
            </div>
          ))}
          <div style={{ display: "flex", gap: 6, marginTop: 8 }}>
            <input className="panel-input" style={{ flex: 1 }} placeholder="Add demo item..." value={newDemoItem} onChange={e => setNewDemoItem(e.target.value)} onKeyDown={e => e.key === "Enter" && addDemoItem()} />
            <button className="panel-btn panel-btn-primary" onClick={addDemoItem}>Add</button>
          </div>
        </div>
      )}

      {/* Retrospective */}
      {subTab === "retro" && (
        <div>
          <div style={sectionTitle}>Retrospective</div>
          <div style={{ display: "flex", gap: 8, marginBottom: 12, alignItems: "center" }}>
            <input className="panel-input" style={{ flex: 1 }} placeholder="Add a card..." value={newRetroText} onChange={e => setNewRetroText(e.target.value)} />
            <button className="panel-btn panel-btn-primary" style={{ whiteSpace: "nowrap", fontSize: "var(--font-size-base)" }} onClick={async () => {
              try {
                const sprintData = await invoke("agile_get_sprints");
                const result = await invoke<{ well: string[]; didnt: string[]; actions: string[] }>("agile_ai_retro_generate", { sprintData , provider});
                const newCards: RetroCard[] = [
                  ...(result.well || []).map(t => ({ id: genId(), text: t, category: "well" as const })),
                  ...(result.didnt || []).map(t => ({ id: genId(), text: t, category: "didnt" as const })),
                  ...(result.actions || []).map(t => ({ id: genId(), text: t, category: "action" as const })),
                ];
                const next = [...retro, ...newCards];
                setRetro(next);
                saveCeremony({ retro: next });
              } catch (_e: any) {
                setError(typeof _e === "string" ? _e : "AI retro generation failed");
              }
            }}>AI Generate Cards</button>
          </div>
          <div style={{ display: "flex", gap: 12 }}>
            {(["well", "didnt", "action"] as const).map(cat => {
              const title = cat === "well" ? "What went well" : cat === "didnt" ? "What didn't go well" : "Action items";
              const color = cat === "well" ? "var(--success-color)" : cat === "didnt" ? "var(--error-color)" : "var(--accent-blue)";
              return (
                <div key={cat} style={{ flex: 1, background: "var(--bg-secondary)", borderRadius: "var(--radius-md)", padding: 10, border: "1px solid var(--border-color)" }}>
                  <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", color, marginBottom: 8, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                    {title}
                    <button className="panel-btn panel-btn-secondary" style={{ padding: "2px 8px", fontSize: "var(--font-size-sm)" }} onClick={() => addRetroCard(cat)}>+</button>
                  </div>
                  {retro.filter(r => r.category === cat).map(r => (
                    <div key={r.id} style={{ ...cardBaseStyle, fontSize: "var(--font-size-base)" }}>{r.text}</div>
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
        const raw = await invoke<any>("agile_get_metrics");
        // Backend returns flat shape — normalize to MetricsData
        const velocities: number[] = raw.velocities || [];
        const data: MetricsData = {
          velocityHistory: raw.velocityHistory || velocities.map((pts: number, i: number) => ({ sprint: `S${i + 1}`, points: pts })),
          cumulativeFlow: raw.cumulativeFlow || [],
          cycleTimeDays: raw.cycleTimeDays ?? raw.avg_velocity ?? 0,
          leadTimeDays: raw.leadTimeDays ?? 0,
          scopeCreepPct: raw.scopeCreepPct ?? 0,
          plannedVsCompleted: raw.plannedVsCompleted ?? (raw.avg_velocity ? 0.8 : 0),
          capacityUtilization: raw.capacityUtilization ?? 0,
        };
        setMetrics(data);
      } catch (_e: any) {
        setError(typeof _e === "string" ? _e : _e?.message || "Failed to load metrics");
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
          <div key={label} style={{ background: "var(--bg-elevated)", borderRadius: "var(--radius-md)", padding: "12px 20px", border: "1px solid var(--border-color)", minWidth: 130, boxShadow: "var(--card-shadow)" }}>
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>{label}</div>
            <div style={{ fontSize: 20, fontWeight: 700, fontFamily: "var(--font-mono)", color }}>{value}</div>
          </div>
        ))}
      </div>

      {/* Velocity chart */}
      <div style={sectionTitle}>Velocity (last {metrics.velocityHistory.length} sprints)</div>
      <div style={{ display: "flex", gap: 8, alignItems: "flex-end", height: 120, marginBottom: 16, padding: "0 4px" }}>
        {metrics.velocityHistory.map(v => (
          <div key={v.sprint} style={{ flex: 1, display: "flex", flexDirection: "column", alignItems: "center" }}>
            <div style={{ fontSize: "var(--font-size-sm)", fontWeight: 600, color: "var(--text-primary)", marginBottom: 2 }}>{v.points}</div>
            <div style={{
              width: "100%",
              maxWidth: 48,
              height: `${(v.points / maxVel) * 100}px`,
              background: "var(--accent-blue)",
              borderRadius: "var(--radius-sm) var(--radius-sm) 0 0",
              transition: "var(--transition-smooth)",
            }} />
            <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", marginTop: 4 }}>{v.sprint}</div>
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
              <span style={{ width: 60, fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>{row.date}</span>
              <div style={{ flex: 1, display: "flex", height: 18, borderRadius: "var(--radius-sm)", overflow: "hidden" }}>
                <div style={{ width: pct(row.backlog), background: "var(--text-secondary)", transition: "var(--transition-smooth)" }} title={`Backlog: ${row.backlog}`} />
                <div style={{ width: pct(row.todo), background: "var(--info-color)", transition: "var(--transition-smooth)" }} title={`To Do: ${row.todo}`} />
                <div style={{ width: pct(row.inProgress), background: "var(--warning-color)", transition: "var(--transition-smooth)" }} title={`In Progress: ${row.inProgress}`} />
                <div style={{ width: pct(row.inReview), background: "var(--accent-purple)", transition: "var(--transition-smooth)" }} title={`In Review: ${row.inReview}`} />
                <div style={{ width: pct(row.done), background: "var(--success-color)", transition: "var(--transition-smooth)" }} title={`Done: ${row.done}`} />
              </div>
            </div>
          );
        })}
        <div style={{ display: "flex", gap: 12, marginTop: 6, fontSize: "var(--font-size-sm)" }}>
          {[
            { label: "Backlog", color: "var(--text-secondary)" },
            { label: "To Do", color: "var(--info-color)" },
            { label: "In Progress", color: "var(--warning-color)" },
            { label: "In Review", color: "var(--accent-purple)" },
            { label: "Done", color: "var(--success-color)" },
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
          <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>Planned vs Completed Ratio</div>
          <div style={{ height: 8, background: "var(--bg-tertiary)", borderRadius: "var(--radius-xs-plus)", marginTop: 6 }}>
            <div style={{ height: "100%", width: `${Math.min(metrics.plannedVsCompleted * 100, 100)}%`, background: metrics.plannedVsCompleted >= 0.8 ? "var(--success-color)" : "var(--warning-color)", borderRadius: "var(--radius-xs-plus)", transition: "var(--transition-smooth)" }} />
          </div>
        </div>
        <div style={{ ...cardBaseStyle, flex: 1, minWidth: 180 }}>
          <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>Scope Creep</div>
          <div style={{ height: 8, background: "var(--bg-tertiary)", borderRadius: "var(--radius-xs-plus)", marginTop: 6 }}>
            <div style={{ height: "100%", width: `${Math.min(metrics.scopeCreepPct, 100)}%`, background: metrics.scopeCreepPct > 20 ? "var(--error-color)" : "var(--success-color)", borderRadius: "var(--radius-xs-plus)", transition: "var(--transition-smooth)" }} />
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
          <button key={m.name} className={`panel-tab ${selected === m.name ? "active" : ""}`} onClick={() => setSelected(m.name)}>
            {m.name}
          </button>
        ))}
        <button className="panel-btn panel-btn-secondary" style={{ marginLeft: "auto" }} onClick={() => setShowCompare(!showCompare)}>
          {showCompare ? "Hide" : "Show"} Comparison
        </button>
      </div>

      {/* Comparison matrix */}
      {showCompare && (
        <div style={{ overflowX: "auto", marginBottom: 16 }}>
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "var(--font-size-base)" }}>
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
        <p style={{ fontSize: "var(--font-size-md)", color: "var(--text-secondary)", lineHeight: 1.5, marginBottom: 12 }}>{info.description}</p>

        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
          <div>
            <div style={sectionTitle}>Core Principles</div>
            <ul style={{ margin: 0, paddingLeft: 16, fontSize: "var(--font-size-base)", color: "var(--text-primary)" }}>
              {info.principles.map(p => <li key={p} style={{ marginBottom: 4 }}>{p}</li>)}
            </ul>
          </div>
          <div>
            <div style={sectionTitle}>Roles</div>
            <ul style={{ margin: 0, paddingLeft: 16, fontSize: "var(--font-size-base)", color: "var(--text-primary)" }}>
              {info.roles.map(r => <li key={r} style={{ marginBottom: 4 }}>{r}</li>)}
            </ul>
          </div>
          <div>
            <div style={sectionTitle}>Ceremonies / Practices</div>
            {info.ceremonies.map(c => (
              <div key={c} role="checkbox" aria-checked={enabledPractices[`${info.name}:${c}`] !== false} tabIndex={0} style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4, cursor: "pointer" }} onClick={() => togglePractice(`${info.name}:${c}`)} onKeyDown={e => e.key === "Enter" && togglePractice(`${info.name}:${c}`)}>
                <span style={{ fontSize: "var(--font-size-lg)", color: enabledPractices[`${info.name}:${c}`] !== false ? "var(--success-color)" : "var(--text-secondary)" }}>
                  {enabledPractices[`${info.name}:${c}`] !== false ? "[x]" : "[ ]"}
                </span>
                <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-primary)" }}>{c}</span>
              </div>
            ))}
          </div>
          <div>
            <div style={sectionTitle}>Artifacts</div>
            <ul style={{ margin: 0, paddingLeft: 16, fontSize: "var(--font-size-base)", color: "var(--text-primary)" }}>
              {info.artifacts.map(a => <li key={a} style={{ marginBottom: 4 }}>{a}</li>)}
            </ul>
          </div>
        </div>

        <div style={{ marginTop: 12, padding: "8px 12px", background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)" }}>
          <span style={{ fontSize: "var(--font-size-base)", fontWeight: 600, color: "var(--accent-gold)" }}>Best Suited For: </span>
          <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>{info.bestFor}</span>
        </div>
      </div>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════════════════
   AI Coach Tab
   ═══════════════════════════════════════════════════════════════════════ */

function AiCoachTab({ provider }: { provider?: string } = {}) {
  const [sprintId, setSprintId] = useState("");
  const [sprints, setSprints] = useState<Sprint[]>([]);
  const [analysis, setAnalysis] = useState<AiAnalysis | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const cancelRef = useRef(false);
  const taskIdRef = useRef<string | null>(null);

  useEffect(() => {
    (async () => {
      try {
        const data = await invoke<{ sprints: Sprint[] }>("agile_get_sprints");
        setSprints(data.sprints || []);
      } catch { /* ignore */ }
    })();
  }, []);

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
      const result = await invoke<AiAnalysis>("agile_ai_analyze", { sprintId: sprintId.trim() , provider});
      if (cancelRef.current) return;
      taskIdRef.current = result.taskId;
      setAnalysis(result);
    } catch (_e: any) {
      if (!cancelRef.current) {
        setError(typeof _e === "string" ? _e : _e?.message || "AI analysis failed");
      }
    } finally {
      if (!cancelRef.current) {
        setLoading(false);
      }
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sprintId]);

  const handleSuspend = useCallback(() => {
    cancelRef.current = true;
    taskIdRef.current = null;
    setLoading(false);
    setError("Analysis suspended by user");
  }, []);

  return (
    <div>
      {error && <div style={{ color: "var(--error-color)", marginBottom: 8, fontSize: "var(--font-size-base)" }}>{error}</div>}

      <div style={{ display: "flex", gap: 8, marginBottom: 16, alignItems: "center" }}>
        <select className="panel-select" style={{ width: 200 }} value={sprintId} onChange={e => setSprintId(e.target.value)}>
          <option value="">Select Sprint...</option>
          {sprints.map(s => <option key={s.id} value={s.id}>{s.name} ({s.status})</option>)}
        </select>
        <button className="panel-btn panel-btn-primary" onClick={analyzesprint} disabled={loading || !sprintId}>
          {loading ? "Analyzing..." : "Analyze Sprint"}
        </button>
        {loading && (
          <button className="panel-btn panel-btn-danger" onClick={handleSuspend}>
            Suspend
          </button>
        )}
      </div>

      {loading && (
        <div style={{ textAlign: "center", padding: 24, color: "var(--text-secondary)" }}>
          <div style={{ fontSize: "var(--font-size-md)", marginBottom: 4 }}>Running AI analysis on sprint data...</div>
          <div style={{ fontSize: "var(--font-size-base)" }}>This may take a moment. You can suspend at any time.</div>
        </div>
      )}

      {analysis && (
        <div>
          {/* Recommendations */}
          <div style={sectionTitle}>Recommendations</div>
          {analysis.recommendations.map((rec, i) => (
            <div key={i} style={{ ...cardBaseStyle, borderLeft: `3px solid ${riskColor(rec.risk)}` }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                <span style={{ fontWeight: 600, fontSize: "var(--font-size-md)", color: "var(--text-primary)" }}>{rec.title}</span>
                <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
                  <span style={badgeStyle("var(--bg-tertiary)", "var(--text-secondary)")}>{rec.category}</span>
                  <span style={{ width: 10, height: 10, borderRadius: "50%", background: riskColor(rec.risk), display: "inline-block" }} title={`Risk: ${rec.risk}`} />
                </div>
              </div>
              <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 6 }}>{rec.description}</div>
              {rec.actionItems.length > 0 && (
                <div>
                  <div style={{ fontSize: "var(--font-size-sm)", fontWeight: 600, color: "var(--text-secondary)", marginBottom: 2 }}>Action Items:</div>
                  <ul style={{ margin: 0, paddingLeft: 16, fontSize: "var(--font-size-base)", color: "var(--text-primary)" }}>
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
                <div key={i} style={{ ...cardBaseStyle, borderLeft: "3px solid var(--warning-color)", padding: "8px 12px", fontSize: "var(--font-size-base)", color: "var(--text-primary)" }}>{b}</div>
              ))}
            </div>
          )}

          {/* Story sizing */}
          {analysis.sizingSuggestions.length > 0 && (
            <div style={{ marginTop: 16 }}>
              <div style={sectionTitle}>Story Sizing Suggestions</div>
              {analysis.sizingSuggestions.map((s, i) => (
                <div key={i} style={{ ...cardBaseStyle, borderLeft: "3px solid var(--accent-blue)", padding: "8px 12px", fontSize: "var(--font-size-base)", color: "var(--text-primary)" }}>{s}</div>
              ))}
            </div>
          )}

          {/* Retro insights */}
          {analysis.retroInsights.length > 0 && (
            <div style={{ marginTop: 16 }}>
              <div style={sectionTitle}>Retrospective Insights</div>
              {analysis.retroInsights.map((r, i) => (
                <div key={i} style={{ ...cardBaseStyle, borderLeft: "3px solid var(--accent-purple)", padding: "8px 12px", fontSize: "var(--font-size-base)", color: "var(--text-primary)" }}>{r}</div>
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
                    <div style={{ width: 24, height: 24, borderRadius: "50%", background: riskColor(level), margin: "0 auto 8px" }} />
                    <div style={{ fontSize: 18, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{count}</div>
                    <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", textTransform: "capitalize" }}>{level}</div>
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
          <div style={{ fontSize: "var(--font-size-lg)", marginBottom: 4 }}>AI Agile Coach</div>
          <div style={{ fontSize: "var(--font-size-base)" }}>Enter a Sprint ID and click "Analyze Sprint" to get AI-powered coaching recommendations, bottleneck detection, and process improvement suggestions.</div>
        </div>
      )}
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════════════════
   SAFe Tab — Full operational SAFe support
   ═══════════════════════════════════════════════════════════════════════ */

function SAFeTab({ provider }: { provider?: string } = {}) {
  const [safeData, setSafeData] = useState<SAFeData>({ programIncrements: [], teams: [], epics: [] });
  const [subView, setSubView] = useState<"pi" | "art" | "portfolio" | "board">("pi");
  const [loading, setLoading] = useState(true);
  const cancelRef = useRef(false);
  const taskIdRef = useRef(0);

  const load = useCallback(async () => {
    const id = ++taskIdRef.current;
    setLoading(true);
    try {
      const data = await invoke<SAFeData>("agile_get_safe");
      if (cancelRef.current || id !== taskIdRef.current) return;
      setSafeData(data);
    } catch { /* first run — empty */ }
    if (!cancelRef.current && id === taskIdRef.current) {
      setLoading(false);
    }
  }, []);

  useEffect(() => { cancelRef.current = false; load(); return () => { cancelRef.current = true; }; }, [load]);

  const save = async (d: SAFeData) => {
    setSafeData(d);
    await invoke("agile_save_safe", { data: d });
  };

  const wsjf = (f: Feature) => f.jobSize > 0 ? ((f.businessValue + f.timeCriticality + f.riskReduction) / f.jobSize) : 0;

  /* ── PI Planning sub-view ── */
  const PIPlanning = () => {
    const [name, setName] = useState("");
    const [iterations, setIterations] = useState(5);

    const createPI = () => {
      if (!name.trim()) return;
      const pi: ProgramIncrement = {
        id: `pi-${Date.now()}`, name: name.trim(), startDate: new Date().toISOString().slice(0, 10),
        endDate: new Date(Date.now() + iterations * 14 * 86400000).toISOString().slice(0, 10),
        status: "Planning", iterations, ipIteration: true, objectives: [], features: [],
      };
      save({ ...safeData, programIncrements: [...safeData.programIncrements, pi] });
      setName("");
    };

    const updatePIStatus = (piId: string, status: PIStatus) => {
      save({ ...safeData, programIncrements: safeData.programIncrements.map(p => p.id === piId ? { ...p, status } : p) });
    };

    const [featureTitle, setFeatureTitle] = useState("");
    const [showFeatureForm, setShowFeatureForm] = useState<string | null>(null);
    const [featureLoading, setFeatureLoading] = useState(false);

    const addFeature = (piId: string) => {
      if (!featureTitle.trim()) { setShowFeatureForm(piId); return; }
      const teamId = safeData.teams.length > 0 ? safeData.teams[0].id : "unassigned";
      const f: Feature = { id: `feat-${Date.now()}`, title: featureTitle.trim(), description: "", teamId, iteration: 1, businessValue: 5, timeCriticality: 5, riskReduction: 5, jobSize: 5, status: "To Do" };
      save({ ...safeData, programIncrements: safeData.programIncrements.map(p => p.id === piId ? { ...p, features: [...p.features, f] } : p) });
      setFeatureTitle("");
      setShowFeatureForm(null);
    };

    const aiGenerateFeatures = async (piId: string) => {
      const pi = safeData.programIncrements.find(p => p.id === piId);
      if (!pi) return;
      setFeatureLoading(true);
      try {
        const objectives = pi.objectives.map(o => o.description).join("; ");
        const result = await invoke<{ features: { title: string; description: string; businessValue: number; timeCriticality: number; riskReduction: number; jobSize: number }[] }>("agile_ai_enhance_safe", { piId, objectiveText: objectives || pi.name , provider});
        if (result.features?.length) {
          const teamId = safeData.teams.length > 0 ? safeData.teams[0].id : "unassigned";
          const newFeatures: Feature[] = result.features.map((f, i) => ({
            id: `feat-${Date.now()}-${i}`, title: f.title, description: f.description || "",
            teamId, iteration: 1, businessValue: f.businessValue || 5, timeCriticality: f.timeCriticality || 5,
            riskReduction: f.riskReduction || 5, jobSize: f.jobSize || 5, status: "To Do" as Column,
          }));
          save({ ...safeData, programIncrements: safeData.programIncrements.map(p => p.id === piId ? { ...p, features: [...p.features, ...newFeatures] } : p) });
        }
      } catch (_e: any) {
        // silently fail — user can add manually
      } finally {
        setFeatureLoading(false);
      }
    };

    const [objectiveDesc, setObjectiveDesc] = useState("");
    const [showObjectiveForm, setShowObjectiveForm] = useState<string | null>(null);

    const addObjective = (piId: string) => {
      if (!objectiveDesc.trim()) { setShowObjectiveForm(piId); return; }
      const teamId = safeData.teams.length > 0 ? safeData.teams[0].id : "unassigned";
      const obj: PIObjective = { id: `obj-${Date.now()}`, teamId, description: objectiveDesc.trim(), businessValue: 5, committed: true, achieved: false };
      save({ ...safeData, programIncrements: safeData.programIncrements.map(p => p.id === piId ? { ...p, objectives: [...p.objectives, obj] } : p) });
      setObjectiveDesc("");
      setShowObjectiveForm(null);
    };

    const PI_STATUSES: PIStatus[] = ["Planning", "Executing", "IP", "Completed"];

    return (
      <div>
        <div className="panel-card" style={{ marginBottom: 12 }}>
          <div style={{ fontWeight: 600, marginBottom: 8 }}>Create Program Increment</div>
          <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
            <input className="panel-input panel-input-full" placeholder="PI name (e.g. PI 24.1)" value={name} onChange={e => setName(e.target.value)} />
            <label style={{ fontSize: "var(--font-size-base)" }}>Iterations: <input type="number" min={2} max={12} value={iterations} onChange={e => setIterations(+e.target.value)} className="panel-input" style={{ width: 60 }} /></label>
            <button className="panel-btn panel-btn-secondary" onClick={createPI}>Create PI</button>
          </div>
        </div>
        {safeData.programIncrements.map(pi => (
          <div key={pi.id} className="panel-card" style={{ marginBottom: 12 }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
              <div style={{ fontWeight: 600, fontSize: "var(--font-size-lg)" }}>{pi.name}</div>
              <div style={{ display: "flex", gap: 4 }}>
                {PI_STATUSES.map(s => (
                  <button key={s} className={`panel-btn panel-tab ${pi.status === s ? "active" : ""}`} style={{ padding: "2px 8px", fontSize: "var(--font-size-sm)" }} onClick={() => updatePIStatus(pi.id, s)}>{s}</button>
                ))}
              </div>
            </div>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 8 }}>{pi.startDate} → {pi.endDate} · {pi.iterations} iterations {pi.ipIteration ? "(+IP)" : ""}</div>
            <div style={{ marginBottom: 8 }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 4 }}>
                <span style={{ fontWeight: 600, fontSize: "var(--font-size-base)" }}>Features ({pi.features.length})</span>
                <div style={{ display: "flex", gap: 4 }}>
                  <button className="panel-btn panel-btn-secondary" style={{ padding: "2px 8px", fontSize: "var(--font-size-sm)" }} onClick={() => addFeature(pi.id)}>+ Feature</button>
                  <button className="panel-btn panel-btn-primary" style={{ padding: "2px 8px", fontSize: "var(--font-size-sm)" }} onClick={() => aiGenerateFeatures(pi.id)} disabled={featureLoading}>
                    {featureLoading ? "Generating..." : "AI Generate"}
                  </button>
                </div>
              </div>
              {showFeatureForm === pi.id && (
                <div style={{ display: "flex", gap: 4, marginBottom: 8 }}>
                  <input className="panel-input panel-input-full" style={{ fontSize: "var(--font-size-base)" }} placeholder="Feature title" value={featureTitle} onChange={e => setFeatureTitle(e.target.value)} onKeyDown={e => e.key === "Enter" && addFeature(pi.id)} autoFocus />
                  <button className="panel-btn panel-btn-primary" style={{ padding: "2px 12px", fontSize: "var(--font-size-sm)" }} onClick={() => addFeature(pi.id)}>Add</button>
                  <button className="panel-btn panel-btn-secondary" style={{ padding: "2px 12px", fontSize: "var(--font-size-sm)" }} onClick={() => { setShowFeatureForm(null); setFeatureTitle(""); }}>Cancel</button>
                </div>
              )}
              {pi.features.sort((a, b) => wsjf(b) - wsjf(a)).map(f => (
                <div key={f.id} style={{ padding: "4px 8px", borderRadius: "var(--radius-xs-plus)", background: "var(--bg-tertiary)", marginBottom: 4, fontSize: "var(--font-size-base)", display: "flex", justifyContent: "space-between" }}>
                  <span>{f.title}</span>
                  <span style={{ color: "var(--text-secondary)" }}>WSJF: {wsjf(f).toFixed(1)} · Team: {safeData.teams.find(t => t.id === f.teamId)?.name || f.teamId}</span>
                </div>
              ))}
            </div>
            <div>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 4 }}>
                <span style={{ fontWeight: 600, fontSize: "var(--font-size-base)" }}>PI Objectives ({pi.objectives.length})</span>
                <button className="panel-btn panel-btn-secondary" style={{ padding: "2px 8px", fontSize: "var(--font-size-sm)" }} onClick={() => addObjective(pi.id)}>+ Objective</button>
              </div>
              {showObjectiveForm === pi.id && (
                <div style={{ display: "flex", gap: 4, marginBottom: 8 }}>
                  <input className="panel-input panel-input-full" style={{ fontSize: "var(--font-size-base)" }} placeholder="PI Objective description" value={objectiveDesc} onChange={e => setObjectiveDesc(e.target.value)} onKeyDown={e => e.key === "Enter" && addObjective(pi.id)} autoFocus />
                  <button className="panel-btn panel-btn-primary" style={{ padding: "2px 12px", fontSize: "var(--font-size-sm)" }} onClick={() => addObjective(pi.id)}>Add</button>
                  <button className="panel-btn panel-btn-secondary" style={{ padding: "2px 12px", fontSize: "var(--font-size-sm)" }} onClick={() => { setShowObjectiveForm(null); setObjectiveDesc(""); }}>Cancel</button>
                </div>
              )}
              {pi.objectives.map(obj => (
                <div key={obj.id} style={{ padding: "4px 8px", borderRadius: "var(--radius-xs-plus)", background: "var(--bg-tertiary)", marginBottom: 4, fontSize: "var(--font-size-base)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <span>{obj.description}</span>
                  <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                    <span style={{ color: "var(--text-secondary)" }}>BV: {obj.businessValue}</span>
                    <label style={{ fontSize: "var(--font-size-sm)" }}><input type="checkbox" checked={obj.achieved} onChange={() => {
                      const newObjs = pi.objectives.map(o => o.id === obj.id ? { ...o, achieved: !o.achieved } : o);
                      save({ ...safeData, programIncrements: safeData.programIncrements.map(p => p.id === pi.id ? { ...p, objectives: newObjs } : p) });
                    }} /> Achieved</label>
                  </div>
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>
    );
  };

  /* ── ART sub-view ── */
  const ARTView = () => {
    const [teamName, setTeamName] = useState("");
    const [teamCapacity, setTeamCapacity] = useState(40);
    const [teamMembers, setTeamMembers] = useState(8);

    const addTeam = () => {
      if (!teamName.trim()) return;
      const team: AgileReleaseTrainTeam = { id: `team-${Date.now()}`, name: teamName.trim(), capacity: teamCapacity, members: teamMembers, features: [] };
      save({ ...safeData, teams: [...safeData.teams, team] });
      setTeamName("");
    };

    const removeTeam = (id: string) => save({ ...safeData, teams: safeData.teams.filter(t => t.id !== id) });

    return (
      <div>
        <div className="panel-card" style={{ marginBottom: 12 }}>
          <div style={{ fontWeight: 600, marginBottom: 8 }}>Add Team to ART</div>
          <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
            <input className="panel-input panel-input-full" placeholder="Team name" value={teamName} onChange={e => setTeamName(e.target.value)} />
            <label style={{ fontSize: "var(--font-size-base)" }}>Capacity: <input type="number" min={10} max={200} value={teamCapacity} onChange={e => setTeamCapacity(+e.target.value)} className="panel-input" style={{ width: 60 }} /></label>
            <label style={{ fontSize: "var(--font-size-base)" }}>Members: <input type="number" min={3} max={15} value={teamMembers} onChange={e => setTeamMembers(+e.target.value)} className="panel-input" style={{ width: 60 }} /></label>
            <button className="panel-btn panel-btn-secondary" onClick={addTeam}>Add Team</button>
          </div>
        </div>
        <div style={{ fontWeight: 600, fontSize: "var(--font-size-lg)", marginBottom: 8 }}>Agile Release Train ({safeData.teams.length} teams)</div>
        {safeData.teams.map(team => {
          const totalLoad = safeData.programIncrements.flatMap(p => p.features).filter(f => f.teamId === team.id).length;
          const loadPct = team.capacity > 0 ? Math.min(100, (totalLoad / team.capacity) * 100 * 10) : 0;
          return (
            <div key={team.id} className="panel-card" style={{ marginBottom: 8 }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 4 }}>
                <span style={{ fontWeight: 600 }}>{team.name}</span>
                <button onClick={() => removeTeam(team.id)} className="panel-btn panel-btn-danger" style={{ padding: "2px 8px", fontSize: "var(--font-size-sm)" }}>Remove</button>
              </div>
              <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 4 }}>{team.members} members · Capacity: {team.capacity} pts · Features: {totalLoad}</div>
              <div style={{ height: 6, borderRadius: 3, background: "var(--bg-tertiary)", overflow: "hidden" }}>
                <div style={{ height: "100%", width: `${loadPct}%`, borderRadius: 3, background: loadPct > 85 ? "var(--error-color)" : loadPct > 60 ? "var(--warning-color)" : "var(--success-color)", transition: "width 0.3s" }} />
              </div>
            </div>
          );
        })}
      </div>
    );
  };

  /* ── Portfolio Kanban sub-view ── */
  const PortfolioKanban = () => {
    const EPIC_COLUMNS: EpicStatus[] = ["Funnel", "Analyzing", "Backlog", "Implementing", "Done"];

    const [epicTitle, setEpicTitle] = useState("");
    const [showEpicForm, setShowEpicForm] = useState(false);

    const addEpic = () => {
      if (!epicTitle.trim()) { setShowEpicForm(true); return; }
      const epic: Epic = { id: `epic-${Date.now()}`, title: epicTitle.trim(), description: "", status: "Funnel", leanBusinessCase: "", wsjfScore: 0, features: [] };
      save({ ...safeData, epics: [...safeData.epics, epic] });
      setEpicTitle("");
      setShowEpicForm(false);
    };

    const moveEpic = (id: string, status: EpicStatus) => {
      save({ ...safeData, epics: safeData.epics.map(e => e.id === id ? { ...e, status } : e) });
    };

    const removeEpic = (id: string) => save({ ...safeData, epics: safeData.epics.filter(e => e.id !== id) });

    return (
      <div>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
          <div style={{ fontWeight: 600, fontSize: "var(--font-size-lg)" }}>Portfolio Kanban</div>
          <button className="panel-btn panel-btn-secondary" onClick={() => setShowEpicForm(true)}>+ Epic</button>
        </div>
        {showEpicForm && (
          <div style={{ display: "flex", gap: 4, marginBottom: 12 }}>
            <input className="panel-input panel-input-full" style={{ fontSize: "var(--font-size-base)" }} placeholder="Epic title" value={epicTitle} onChange={e => setEpicTitle(e.target.value)} onKeyDown={e => e.key === "Enter" && addEpic()} autoFocus />
            <button className="panel-btn panel-btn-primary" style={{ padding: "4px 12px", fontSize: "var(--font-size-sm)" }} onClick={addEpic}>Add</button>
            <button className="panel-btn panel-btn-secondary" style={{ padding: "4px 12px", fontSize: "var(--font-size-sm)" }} onClick={() => { setShowEpicForm(false); setEpicTitle(""); }}>Cancel</button>
          </div>
        )}
        <div style={{ display: "grid", gridTemplateColumns: `repeat(${EPIC_COLUMNS.length}, 1fr)`, gap: 8 }}>
          {EPIC_COLUMNS.map(col => (
            <div key={col} style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-sm-alt)", padding: 8, minHeight: 120 }}>
              <div style={{ fontWeight: 600, fontSize: "var(--font-size-base)", marginBottom: 8, textAlign: "center", color: "var(--text-secondary)" }}>{col} ({safeData.epics.filter(e => e.status === col).length})</div>
              {safeData.epics.filter(e => e.status === col).map(epic => (
                <div key={epic.id} style={{ padding: 8, borderRadius: "var(--radius-sm)", background: "var(--bg-primary)", marginBottom: 6, border: "1px solid var(--border-color)" }}>
                  <div style={{ fontWeight: 600, fontSize: "var(--font-size-base)", marginBottom: 4 }}>{epic.title}</div>
                  {epic.wsjfScore > 0 && <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 4 }}>WSJF: {epic.wsjfScore}</div>}
                  <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
                    {EPIC_COLUMNS.filter(c => c !== col).map(c => (
                      <button key={c} className="panel-btn panel-btn-secondary" style={{ padding: "1px 8px", fontSize: "var(--font-size-xs)" }} onClick={() => moveEpic(epic.id, c)}>→ {c}</button>
                    ))}
                    <button className="panel-btn panel-btn-danger" style={{ padding: "1px 8px", fontSize: "var(--font-size-xs)" }} onClick={() => removeEpic(epic.id)} aria-label={`Delete epic ${epic.title}`}><X size={10} /></button>
                  </div>
                </div>
              ))}
            </div>
          ))}
        </div>
      </div>
    );
  };

  /* ── Program Board sub-view ── */
  const ProgramBoard = () => {
    const activePIs = safeData.programIncrements.filter(p => p.status !== "Completed");
    const pi = activePIs.length > 0 ? activePIs[0] : null;
    if (!pi) return <div style={{ fontSize: "var(--font-size-md)", color: "var(--text-secondary)" }}>No active Program Increment. Create one in PI Planning.</div>;

    const iterationNums = Array.from({ length: pi.iterations + (pi.ipIteration ? 1 : 0) }, (_, i) => i + 1);

    return (
      <div>
        <div style={{ fontWeight: 600, fontSize: "var(--font-size-lg)", marginBottom: 8 }}>Program Board — {pi.name}</div>
        <div style={{ overflowX: "auto" }}>
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "var(--font-size-base)" }}>
            <thead>
              <tr>
                <th style={{ padding: 6, borderBottom: "2px solid var(--border-color)", textAlign: "left" }}>Team</th>
                {iterationNums.map(i => (
                  <th key={i} style={{ padding: 6, borderBottom: "2px solid var(--border-color)", textAlign: "center" }}>
                    {i <= pi.iterations ? `Iter ${i}` : "IP"}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {safeData.teams.map(team => (
                <tr key={team.id}>
                  <td style={{ padding: 6, borderBottom: "1px solid var(--border-color)", fontWeight: 600 }}>{team.name}</td>
                  {iterationNums.map(iter => {
                    const features = pi.features.filter(f => f.teamId === team.id && f.iteration === iter);
                    return (
                      <td key={iter} style={{ padding: 4, borderBottom: "1px solid var(--border-color)", verticalAlign: "top" }}>
                        {features.map(f => (
                          <div key={f.id} style={{ padding: "2px 8px", borderRadius: "var(--radius-xs-plus)", background: "var(--accent-blue)", color: "var(--btn-primary-fg)", fontSize: "var(--font-size-xs)", marginBottom: 2, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }} title={`${f.title} (WSJF: ${wsjf(f).toFixed(1)})`}>
                            {f.title}
                          </div>
                        ))}
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    );
  };

  const SUB_VIEWS: { key: typeof subView; label: string }[] = [
    { key: "pi", label: "PI Planning" },
    { key: "art", label: "ART" },
    { key: "portfolio", label: "Portfolio Kanban" },
    { key: "board", label: "Program Board" },
  ];

  if (loading) return <div style={{ padding: 16, color: "var(--text-secondary)" }}>Loading SAFe data…</div>;

  return (
    <div>
      <div style={{ display: "flex", gap: 4, marginBottom: 12 }}>
        {SUB_VIEWS.map(sv => (
          <button key={sv.key} className={`panel-btn panel-tab ${subView === sv.key ? "active" : ""}`} style={{ fontSize: "var(--font-size-base)" }} onClick={() => setSubView(sv.key)}>
            {sv.label}
          </button>
        ))}
      </div>
      {subView === "pi" && <PIPlanning />}
      {subView === "art" && <ARTView />}
      {subView === "portfolio" && <PortfolioKanban />}
      {subView === "board" && <ProgramBoard />}
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
  { key: "safe", label: "SAFe" },
  { key: "coach", label: "AI Coach" },
];

function AgilePanel({ provider }: { provider?: string }) {
  const [activeTab, setActiveTab] = useState<TabKey>("board");

  return (
    <div className="panel-container">
      <h2 style={{ margin: "0 0 12px", fontSize: 18, fontWeight: 700, color: "var(--accent-color)" }}>
        Agile Project Management
      </h2>

      {/* Tab bar */}
      <div className="panel-tab-bar">
        {TABS.map(t => (
          <button key={t.key} className={`panel-tab ${activeTab === t.key ? "active" : ""}`} onClick={() => setActiveTab(t.key)}>
            {t.label}
          </button>
        ))}
      </div>

      {/* Tab content — scrollable in both axes */}
      <div style={{ flex: 1, minHeight: 0, overflow: "auto", padding: "12px 0 0" }}>
        {activeTab === "board" && <BoardTab provider={provider} />}
        {activeTab === "sprint" && <SprintTab />}
        {activeTab === "backlog" && <BacklogTab provider={provider} />}
        {activeTab === "ceremonies" && <CeremoniesTab provider={provider} />}
        {activeTab === "metrics" && <MetricsTab />}
        {activeTab === "methodology" && <MethodologyTab />}
        {activeTab === "safe" && <SAFeTab provider={provider} />}
        {activeTab === "coach" && <AiCoachTab provider={provider} />}
      </div>
    </div>
  );
}

export default AgilePanel;
