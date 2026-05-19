import { useEffect, useState, useCallback, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Target, Plus, Play, Link2, Trash2, RefreshCw, Tag, ListTree, FileText, Star } from 'lucide-react';
import { useToast } from '../hooks/useToast';
import { Toaster } from './Toaster';

// ── Types ────────────────────────────────────────────────────────────────────

type GoalStatus = 'active' | 'paused' | 'done' | 'abandoned';
type GoalLinkKind = 'session' | 'job' | 'recap' | 'note';

interface PlanStep {
  id: number;
  description: string;
  tool: string;
  estimated_path?: string | null;
  status: 'pending' | 'in_progress' | 'done' | 'failed' | 'skipped';
}

interface ExecutionPlan {
  goal: string;
  steps: PlanStep[];
  estimated_files: string[];
  risks: string[];
}

interface Goal {
  id: string;
  workspace?: string | null;
  title: string;
  statement: string;
  status: GoalStatus;
  success_criteria: string[];
  tags: string[];
  created_at: string;
  updated_at: string;
  parent_goal_id?: string | null;
  current_plan?: ExecutionPlan | null;
  schema_version: number;
}

interface GoalLink {
  id: string;
  goal_id: string;
  kind: GoalLinkKind;
  target_id: string;
  linked_at: string;
  note?: string | null;
}

interface GoalDetailResponse {
  goal: Goal;
  links: GoalLink[];
}

// G5.4 — aggregate-recap response (heuristic or LLM-synthesized).
interface GoalRecap {
  goal_id: string;
  title: string;
  headline: string;
  bullets: string[];
  next_actions: string[];
  sources: Array<{ recap_id: string; kind: string; target_id: string }>;
  recap_synthesizer: 'heuristic' | 'llm';
  recap_llm_error?: string;
}

interface GoalPanelProps {
  workspacePath: string | null;
  selectedProvider: string;
  selectedModel?: string;
  /** Optional seed text used to pre-fill the New Goal modal when the
   *  user typed `/goal <text>` in the chat input. */
  newGoalSeed?: string | null;
  /** Called after the seed has been consumed so the parent can clear it. */
  onSeedConsumed?: () => void;
}

const STATUS_INTENT: Record<GoalStatus, string> = {
  active: 'panel-tag-success',
  paused: 'panel-tag-info',
  done: 'panel-tag-info',
  abandoned: 'panel-tag-warning',
};

const STATUS_LABEL: Record<GoalStatus, string> = {
  active: 'Active',
  paused: 'Paused',
  done: 'Done',
  abandoned: 'Abandoned',
};

const STEP_ICON: Record<PlanStep['status'], string> = {
  pending: '◯',
  in_progress: '◔',
  done: '●',
  failed: '✕',
  skipped: '–',
};

// ── Helpers ──────────────────────────────────────────────────────────────────

function short(id: string): string {
  return id.slice(0, 8);
}

function workspaceLabel(ws: string | null | undefined): string {
  if (!ws) return 'global';
  const parts = ws.split('/').filter(Boolean);
  return parts[parts.length - 1] ?? ws;
}

// ── Component ────────────────────────────────────────────────────────────────

export function GoalPanel({
  workspacePath,
  selectedProvider,
  selectedModel,
  newGoalSeed,
  onSeedConsumed,
}: GoalPanelProps) {
  const { toasts, toast, dismiss } = useToast();
  const [goals, setGoals] = useState<Goal[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<GoalDetailResponse | null>(null);
  const [statusFilter, setStatusFilter] = useState<GoalStatus | 'all'>('active');
  // G10.1 — debounced keyword search across title + statement. The
  // `searchInput` is what the user types; `searchQuery` is debounced
  // and fed to the daemon's ?q= filter.
  const [searchInput, setSearchInput] = useState('');
  const [searchQuery, setSearchQuery] = useState('');
  const [loading, setLoading] = useState(false);
  const [planning, setPlanning] = useState(false);
  const [starting, setStarting] = useState(false);
  const [showNewModal, setShowNewModal] = useState(false);
  const [newTitle, setNewTitle] = useState('');
  const [newStatement, setNewStatement] = useState('');
  // G5.4 — tree-view + aggregate-recap state.
  const [viewMode, setViewMode] = useState<'list' | 'tree'>('list');
  const [recapResult, setRecapResult] = useState<GoalRecap | null>(null);
  const [recapping, setRecapping] = useState(false);
  // G10.2 — local input for adding a new tag chip. Bound to the
  // inline input in the Tags section; cleared on submit.
  const [newTagInput, setNewTagInput] = useState('');
  // G6.1 — current-pin state. `pinnedGoalId` is null when nothing is
  // pinned for the active workspace (or the global slot when workspace
  // is empty). Refreshed alongside the goal list.
  const [pinnedGoalId, setPinnedGoalId] = useState<string | null>(null);
  const [pinning, setPinning] = useState(false);

  // Re-order the flat goal list into parent → children sequence with
  // a per-row `depth` so the renderer can indent. Roots are goals whose
  // `parent_goal_id` is either null or not present in the current list
  // (a parent filtered out by the status filter is treated as a root).
  const orderedGoals = useMemo(() => {
    if (viewMode === 'list') {
      return goals.map((g) => ({ goal: g, depth: 0 }));
    }
    const byParent = new Map<string | null, Goal[]>();
    const idSet = new Set(goals.map((g) => g.id));
    for (const g of goals) {
      const p = g.parent_goal_id && idSet.has(g.parent_goal_id) ? g.parent_goal_id : null;
      const arr = byParent.get(p) ?? [];
      arr.push(g);
      byParent.set(p, arr);
    }
    const out: Array<{ goal: Goal; depth: number }> = [];
    const walk = (parent: string | null, depth: number) => {
      const kids = byParent.get(parent) ?? [];
      for (const k of kids) {
        out.push({ goal: k, depth });
        walk(k.id, depth + 1);
      }
    };
    walk(null, 0);
    return out;
  }, [goals, viewMode]);

  const refreshList = useCallback(async () => {
    setLoading(true);
    try {
      const args: Record<string, unknown> = {};
      if (statusFilter !== 'all') args.status = statusFilter;
      if (searchQuery.trim().length > 0) args.q = searchQuery.trim();
      const resp = (await invoke('exec_goal_list', args)) as
        | { goals?: Goal[] }
        | Goal[];
      const list = Array.isArray(resp) ? resp : resp?.goals ?? [];
      setGoals(list);
      if (list.length > 0 && !selectedId) {
        setSelectedId(list[0].id);
      }
    } catch (e) {
      toast.error('Failed to list goals: ' + String(e));
    } finally {
      setLoading(false);
    }
  }, [statusFilter, searchQuery, selectedId, toast]);

  // G10.1 — debounce the search input by 200 ms so each keystroke
  // doesn't fire an exec_goal_list invoke. 200 ms is small enough to
  // feel instant on a typing rhythm without thrashing the daemon.
  useEffect(() => {
    const t = setTimeout(() => setSearchQuery(searchInput), 200);
    return () => clearTimeout(t);
  }, [searchInput]);

  const refreshDetail = useCallback(async (id: string) => {
    try {
      const resp = (await invoke('exec_goal_get', { id })) as GoalDetailResponse;
      setDetail(resp);
    } catch (e) {
      toast.error('Failed to load goal: ' + String(e));
      setDetail(null);
    }
  }, [toast]);

  // G6.1 — refresh which goal is pinned for the active workspace.
  // Silent on failure (no toast) — pin is an optional convenience.
  const refreshPin = useCallback(async () => {
    try {
      const resp = (await invoke('exec_goal_current', {
        workspace: workspacePath || null,
      })) as { goal_id?: string | null };
      setPinnedGoalId(resp.goal_id ?? null);
    } catch {
      setPinnedGoalId(null);
    }
  }, [workspacePath]);

  useEffect(() => { refreshList(); }, [refreshList]);
  useEffect(() => { refreshPin(); }, [refreshPin]);

  useEffect(() => {
    if (selectedId) {
      refreshDetail(selectedId);
    } else {
      setDetail(null);
    }
  }, [selectedId, refreshDetail]);

  // Open the New Goal modal when the parent sends a seed (typed via `/goal …`
  // in the chat input).
  useEffect(() => {
    if (newGoalSeed && newGoalSeed.trim().length > 0) {
      setNewTitle(newGoalSeed.trim().slice(0, 120));
      setNewStatement('');
      setShowNewModal(true);
      onSeedConsumed?.();
    }
  }, [newGoalSeed, onSeedConsumed]);

  // ── Mutations ──────────────────────────────────────────────────────────────

  const createGoal = async () => {
    if (!newTitle.trim()) {
      toast.warn('Title required');
      return;
    }
    try {
      const created = (await invoke('exec_goal_create', {
        title: newTitle.trim(),
        statement: newStatement.trim() || null,
        workspace: workspacePath || null,
      })) as Goal;
      setShowNewModal(false);
      setNewTitle('');
      setNewStatement('');
      await refreshList();
      setSelectedId(created.id);
    } catch (e) {
      toast.error('Failed to create goal: ' + String(e));
    }
  };

  const setStatus = async (status: GoalStatus) => {
    if (!detail) return;
    try {
      const updated = (await invoke('exec_goal_update', {
        id: detail.goal.id,
        status,
      })) as Goal;
      setDetail({ goal: updated, links: detail.links });
      await refreshList();
    } catch (e) {
      toast.error('Failed to update status: ' + String(e));
    }
  };

  const generatePlan = async () => {
    if (!detail) return;
    if (!selectedProvider) {
      toast.warn('Select a provider in the toolbar — plan generation routes through it.');
      return;
    }
    setPlanning(true);
    try {
      const updated = (await invoke('exec_goal_plan', {
        id: detail.goal.id,
        provider: selectedProvider,
        model: selectedModel ?? null,
      })) as Goal;
      setDetail({ goal: updated, links: detail.links });
      toast.success('Plan generated');
    } catch (e) {
      toast.error('Plan generation failed: ' + String(e));
    } finally {
      setPlanning(false);
    }
  };

  const startSession = async () => {
    if (!detail) return;
    setStarting(true);
    try {
      const resp = (await invoke('exec_goal_start', {
        id: detail.goal.id,
        provider: selectedProvider || null,
        model: selectedModel || null,
      })) as { session_id: string; link_id: string };
      toast.success(`Session ${short(resp.session_id)} linked to goal`);
      await refreshDetail(detail.goal.id);
    } catch (e) {
      toast.error('Failed to start session: ' + String(e));
    } finally {
      setStarting(false);
    }
  };

  // G6.1 — toggle pin for the active workspace (or global slot when
  // none). Pin replaces any prior pin; unpin clears unconditionally.
  // G9.1 — emit `vibeui:pin-changed` so the chat-tab PinnedGoalBanner
  // picks up the change immediately without waiting for its 15 s poll.
  const togglePin = async () => {
    if (!detail) return;
    setPinning(true);
    try {
      if (pinnedGoalId === detail.goal.id) {
        await invoke('exec_goal_unpin', { workspace: workspacePath || null });
        setPinnedGoalId(null);
        toast.success('Pin cleared');
      } else {
        await invoke('exec_goal_pin', {
          id: detail.goal.id,
          workspace: workspacePath || null,
        });
        setPinnedGoalId(detail.goal.id);
        toast.success(`Pinned ${detail.goal.title} as current goal`);
      }
      window.dispatchEvent(new CustomEvent('vibeui:pin-changed'));
    } catch (e) {
      toast.error('Pin update failed: ' + String(e));
    } finally {
      setPinning(false);
    }
  };

  const runAggregateRecap = async () => {
    if (!detail) return;
    setRecapping(true);
    setRecapResult(null);
    try {
      // Pass provider + model so the daemon takes the LLM-synthesis
      // path when both are populated (CLAUDE.md provider-agnostic rule);
      // omit both → heuristic fold.
      const resp = (await invoke('exec_goal_recap', {
        id: detail.goal.id,
        provider: selectedProvider || null,
        model: selectedModel || null,
      })) as GoalRecap;
      setRecapResult(resp);
      if (resp.recap_llm_error) {
        toast.warn(`LLM synthesis failed; showed heuristic. ${resp.recap_llm_error}`);
      }
    } catch (e) {
      toast.error('Failed to aggregate recap: ' + String(e));
    } finally {
      setRecapping(false);
    }
  };

  // G10.2 — tag mutation helpers. Both replace the full tags array
  // on the server via exec_goal_update so the optimistic update is
  // safe to roll back to the previous state on error.
  const addTag = async () => {
    if (!detail) return;
    const raw = newTagInput.trim();
    if (!raw) return;
    if (detail.goal.tags.includes(raw)) {
      setNewTagInput('');
      return;
    }
    const next = [...detail.goal.tags, raw];
    setNewTagInput('');
    try {
      const updated = (await invoke('exec_goal_update', {
        id: detail.goal.id,
        tags: next,
      })) as Goal;
      setDetail({ goal: updated, links: detail.links });
    } catch (e) {
      toast.error('Failed to add tag: ' + String(e));
    }
  };

  const removeTag = async (t: string) => {
    if (!detail) return;
    const next = detail.goal.tags.filter((x) => x !== t);
    try {
      const updated = (await invoke('exec_goal_update', {
        id: detail.goal.id,
        tags: next,
      })) as Goal;
      setDetail({ goal: updated, links: detail.links });
    } catch (e) {
      toast.error('Failed to remove tag: ' + String(e));
    }
  };

  const deleteGoal = async () => {
    if (!detail) return;
    if (!window.confirm(`Delete goal "${detail.goal.title}"? Links cascade.`)) return;
    try {
      await invoke('exec_goal_delete', { id: detail.goal.id });
      setSelectedId(null);
      await refreshList();
    } catch (e) {
      toast.error('Failed to delete: ' + String(e));
    }
  };

  // ── Render ─────────────────────────────────────────────────────────────────

  return (
    <div className="panel-root" style={{ display: 'flex', height: '100%', overflow: 'hidden' }}>
      <Toaster toasts={toasts} onDismiss={dismiss} />

      {/* Left rail: list */}
      <div
        className="panel-card"
        style={{
          width: 320,
          display: 'flex',
          flexDirection: 'column',
          borderRight: '1px solid var(--border-default)',
          borderRadius: 0,
        }}
      >
        <div style={{ padding: 12, display: 'flex', alignItems: 'center', gap: 8 }}>
          <Target size={16} />
          <strong>Goals</strong>
          <button
            type="button"
            className="panel-btn panel-btn-primary"
            style={{ marginLeft: 'auto' }}
            onClick={() => {
              setNewTitle('');
              setNewStatement('');
              setShowNewModal(true);
            }}
            title="New goal"
          >
            <Plus size={14} />
            New
          </button>
          <button
            type="button"
            className="panel-btn"
            onClick={() => setViewMode(viewMode === 'list' ? 'tree' : 'list')}
            title={viewMode === 'tree' ? 'Switch to flat list' : 'Switch to tree view'}
          >
            <ListTree size={14} />
          </button>
          <button
            type="button"
            className="panel-btn"
            onClick={refreshList}
            title="Refresh"
            disabled={loading}
          >
            <RefreshCw size={14} />
          </button>
        </div>

        <div style={{ padding: '0 12px 8px', display: 'flex', gap: 4, flexWrap: 'wrap' }}>
          {(['active', 'paused', 'done', 'abandoned', 'all'] as const).map((s) => (
            <button
              key={s}
              type="button"
              className={`panel-btn ${statusFilter === s ? 'panel-btn-primary' : ''}`}
              onClick={() => setStatusFilter(s)}
              style={{ fontSize: 'var(--font-size-sm)', padding: '2px 8px' }}
            >
              {s === 'all' ? 'All' : STATUS_LABEL[s]}
            </button>
          ))}
        </div>

        {/* G10.1 — keyword search across title + statement. Debounced
            client-side; the daemon ANDs `?q=` with the status filter
            above so the two work together. */}
        <div style={{ padding: '0 12px 8px' }}>
          <input
            type="search"
            value={searchInput}
            placeholder="Search title or statement…"
            onChange={(e) => setSearchInput(e.target.value)}
            style={{
              width: '100%',
              padding: '4px 8px',
              fontSize: 'var(--font-size-sm)',
              background: 'var(--bg-default)',
              color: 'var(--text-primary)',
              border: '1px solid var(--border-default)',
              borderRadius: 4,
              outline: 'none',
            }}
            aria-label="Search goals"
          />
        </div>

        <div style={{ flex: 1, overflowY: 'auto' }}>
          {goals.length === 0 ? (
            <div className="panel-empty" style={{ padding: 16 }}>
              {loading ? 'Loading…' : 'No goals yet. Click New.'}
            </div>
          ) : (
            <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>
              {orderedGoals.map(({ goal: g, depth }) => {
                const isSelected = g.id === selectedId;
                return (
                  <li
                    key={g.id}
                    onClick={() => setSelectedId(g.id)}
                    style={{
                      padding: '10px 12px',
                      paddingLeft: 12 + depth * 16,
                      cursor: 'pointer',
                      borderBottom: '1px solid var(--border-subtle)',
                      background: isSelected ? 'var(--bg-selected)' : 'transparent',
                    }}
                  >
                    <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
                      <span
                        className={`panel-tag ${STATUS_INTENT[g.status]}`}
                        style={{ fontSize: 'var(--font-size-xs)' }}
                      >
                        {STATUS_LABEL[g.status]}
                      </span>
                      <span style={{ fontSize: 'var(--font-size-xs)', color: 'var(--text-secondary)' }}>
                        {workspaceLabel(g.workspace)}
                      </span>
                    </div>
                    <div style={{ fontSize: 'var(--font-size-md)', display: 'flex', alignItems: 'center', gap: 6 }}>
                      {pinnedGoalId === g.id && (
                        <Star
                          size={12}
                          fill="currentColor"
                          style={{ color: 'var(--accent-primary)', flexShrink: 0 }}
                          aria-label="current pinned goal"
                        />
                      )}
                      <span>{g.title}</span>
                    </div>
                    <div style={{ fontSize: 'var(--font-size-xs)', color: 'var(--text-tertiary)' }}>
                      {short(g.id)} · updated {new Date(g.updated_at).toLocaleDateString()}
                    </div>
                  </li>
                );
              })}
            </ul>
          )}
        </div>
      </div>

      {/* Right pane: detail */}
      <div style={{ flex: 1, overflowY: 'auto', padding: 16 }}>
        {!detail ? (
          <div className="panel-empty" style={{ marginTop: 64, textAlign: 'center' }}>
            <Target size={32} style={{ opacity: 0.4 }} />
            <p>Select a goal to view detail, or click <strong>New</strong> to create one.</p>
          </div>
        ) : (
          <>
            {/* Header */}
            <div style={{ display: 'flex', alignItems: 'flex-start', gap: 12, marginBottom: 12 }}>
              <div style={{ flex: 1 }}>
                <h2 style={{ marginBottom: 4 }}>{detail.goal.title}</h2>
                <div style={{ fontSize: 'var(--font-size-xs)', color: 'var(--text-secondary)' }}>
                  {short(detail.goal.id)} · created {new Date(detail.goal.created_at).toLocaleString()}
                  {detail.goal.workspace ? ` · ${workspaceLabel(detail.goal.workspace)}` : ' · global'}
                </div>
              </div>
              <button
                type="button"
                className={`panel-btn ${pinnedGoalId === detail.goal.id ? 'panel-btn-primary' : ''}`}
                onClick={togglePin}
                disabled={pinning}
                title={
                  pinnedGoalId === detail.goal.id
                    ? 'Unpin — new /agent sessions won\'t auto-link to this goal'
                    : 'Pin as current — new /agent sessions auto-link to this goal'
                }
              >
                <Star
                  size={14}
                  fill={pinnedGoalId === detail.goal.id ? 'currentColor' : 'none'}
                />
                {pinnedGoalId === detail.goal.id ? 'Pinned' : 'Pin'}
              </button>
              <button
                type="button"
                className="panel-btn"
                onClick={deleteGoal}
                title="Delete goal"
              >
                <Trash2 size={14} /> Delete
              </button>
            </div>

            {/* Status switcher */}
            <div style={{ display: 'flex', gap: 4, marginBottom: 16 }}>
              {(['active', 'paused', 'done', 'abandoned'] as const).map((s) => (
                <button
                  key={s}
                  type="button"
                  className={`panel-btn ${detail.goal.status === s ? 'panel-btn-primary' : ''}`}
                  onClick={() => setStatus(s)}
                >
                  {STATUS_LABEL[s]}
                </button>
              ))}
            </div>

            {/* Statement */}
            <section style={{ marginBottom: 16 }}>
              <h3 style={{ marginBottom: 8 }}>Statement</h3>
              <p style={{ whiteSpace: 'pre-wrap', color: 'var(--text-primary)' }}>
                {detail.goal.statement || (
                  <em style={{ color: 'var(--text-tertiary)' }}>
                    No statement yet. Use the daemon (PATCH /v1/goals/{detail.goal.id}) to add detail.
                  </em>
                )}
              </p>
            </section>

            {/* Tags + criteria — G10.2 made Tags inline-editable. */}
            <section style={{ marginBottom: 16 }}>
              <div style={{ marginBottom: 8, display: 'flex', alignItems: 'center', gap: 6, flexWrap: 'wrap' }}>
                <strong style={{ marginRight: 4 }}>Tags:</strong>
                {detail.goal.tags.map((t) => (
                  <span
                    key={t}
                    className="panel-tag"
                    style={{
                      display: 'inline-flex',
                      alignItems: 'center',
                      gap: 4,
                      paddingRight: 4,
                    }}
                  >
                    <Tag size={10} /> {t}
                    <button
                      type="button"
                      onClick={() => removeTag(t)}
                      title={`Remove tag "${t}"`}
                      style={{
                        background: 'none',
                        border: 'none',
                        cursor: 'pointer',
                        padding: 0,
                        color: 'inherit',
                        opacity: 0.7,
                        fontSize: 'var(--font-size-xs)',
                      }}
                    >
                      ×
                    </button>
                  </span>
                ))}
                <input
                  type="text"
                  value={newTagInput}
                  placeholder={detail.goal.tags.length === 0 ? 'add tag…' : '+ tag'}
                  onChange={(e) => setNewTagInput(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') {
                      e.preventDefault();
                      addTag();
                    }
                  }}
                  onBlur={() => {
                    if (newTagInput.trim().length > 0) addTag();
                  }}
                  style={{
                    width: 90,
                    padding: '2px 6px',
                    fontSize: 'var(--font-size-xs)',
                    background: 'var(--bg-default)',
                    color: 'var(--text-primary)',
                    border: '1px solid var(--border-default)',
                    borderRadius: 4,
                    outline: 'none',
                  }}
                  aria-label="Add tag"
                />
              </div>
              {detail.goal.success_criteria.length > 0 && (
                <div>
                  <strong>Success criteria:</strong>
                  <ul style={{ marginTop: 4 }}>
                    {detail.goal.success_criteria.map((c, i) => (
                      <li key={i}>{c}</li>
                    ))}
                  </ul>
                </div>
              )}
            </section>

            {/* Plan */}
            <section style={{ marginBottom: 16 }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
                <h3 style={{ margin: 0 }}>Plan</h3>
                <button
                  type="button"
                  className="panel-btn panel-btn-primary"
                  onClick={generatePlan}
                  disabled={planning}
                  style={{ marginLeft: 'auto' }}
                >
                  <RefreshCw size={14} className={planning ? 'spin' : ''} />
                  {detail.goal.current_plan ? 'Refresh plan' : 'Generate plan'}
                </button>
                <button
                  type="button"
                  className="panel-btn"
                  onClick={startSession}
                  disabled={starting}
                  title="Create a new session bound to this goal"
                >
                  <Play size={14} /> Start session
                </button>
              </div>
              {!detail.goal.current_plan ? (
                <div className="panel-empty">
                  No plan yet. Click <strong>Generate plan</strong> (uses the provider/model selected in the toolbar).
                </div>
              ) : (
                <div className="panel-card" style={{ padding: 12 }}>
                  <ol style={{ paddingLeft: 20 }}>
                    {detail.goal.current_plan.steps.map((step) => (
                      <li key={step.id} style={{ marginBottom: 6 }}>
                        <span style={{ marginRight: 6 }}>{STEP_ICON[step.status]}</span>
                        <code style={{ marginRight: 6 }}>[{step.tool}]</code>
                        {step.description}
                        {step.estimated_path && (
                          <span
                            style={{
                              marginLeft: 6,
                              fontSize: 'var(--font-size-xs)',
                              color: 'var(--text-tertiary)',
                            }}
                          >
                            ({step.estimated_path})
                          </span>
                        )}
                      </li>
                    ))}
                  </ol>
                  {detail.goal.current_plan.estimated_files.length > 0 && (
                    <p style={{ fontSize: 'var(--font-size-sm)', color: 'var(--text-secondary)' }}>
                      Files: {detail.goal.current_plan.estimated_files.join(', ')}
                    </p>
                  )}
                  {detail.goal.current_plan.risks.length > 0 && (
                    <p style={{ fontSize: 'var(--font-size-sm)', color: 'var(--text-warning)' }}>
                      Risks: {detail.goal.current_plan.risks.join('; ')}
                    </p>
                  )}
                </div>
              )}
            </section>

            {/* G5.4 — Aggregate recap */}
            <section style={{ marginBottom: 16 }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
                <h3 style={{ margin: 0 }}>
                  <FileText size={14} style={{ verticalAlign: 'middle' }} /> Aggregate recap
                </h3>
                <button
                  type="button"
                  className="panel-btn panel-btn-primary"
                  onClick={runAggregateRecap}
                  disabled={recapping}
                  style={{ marginLeft: 'auto' }}
                  title={
                    selectedProvider && selectedModel
                      ? 'Synthesize via LLM (uses toolbar provider+model)'
                      : 'Heuristic fold — select provider+model for LLM synthesis'
                  }
                >
                  <RefreshCw size={14} className={recapping ? 'spin' : ''} />
                  {recapping ? 'Generating…' : 'Generate recap'}
                </button>
              </div>
              {recapResult && (
                <div className="panel-card" style={{ padding: 12 }}>
                  <div
                    style={{
                      fontSize: 'var(--font-size-xs)',
                      color: 'var(--text-tertiary)',
                      marginBottom: 6,
                    }}
                  >
                    Synthesizer: <strong>{recapResult.recap_synthesizer}</strong>
                    {' · '}
                    {recapResult.sources.length} source
                    {recapResult.sources.length === 1 ? '' : 's'}
                  </div>
                  <h4 style={{ margin: '4px 0 8px 0' }}>{recapResult.headline}</h4>
                  {recapResult.bullets.length > 0 && (
                    <ul style={{ paddingLeft: 20, marginBottom: 8 }}>
                      {recapResult.bullets.map((b, i) => (
                        <li key={i}>{b}</li>
                      ))}
                    </ul>
                  )}
                  {recapResult.next_actions.length > 0 && (
                    <>
                      <h5 style={{ margin: '8px 0 4px 0' }}>Next actions</h5>
                      <ul style={{ paddingLeft: 20 }}>
                        {recapResult.next_actions.map((n, i) => (
                          <li key={i}>{n}</li>
                        ))}
                      </ul>
                    </>
                  )}
                </div>
              )}
            </section>

            {/* Links */}
            <section>
              <h3 style={{ marginBottom: 8 }}>
                <Link2 size={14} style={{ verticalAlign: 'middle' }} /> Linked sessions / jobs / recaps ({detail.links.length})
              </h3>
              {detail.links.length === 0 ? (
                <div className="panel-empty">
                  No links yet. Click <strong>Start session</strong> above to spawn one.
                </div>
              ) : (
                <table className="panel-table">
                  <thead>
                    <tr>
                      <th>Kind</th>
                      <th>Target</th>
                      <th>Linked</th>
                      <th>Note</th>
                    </tr>
                  </thead>
                  <tbody>
                    {detail.links.map((l) => (
                      <tr key={l.id}>
                        <td>
                          <span className="panel-tag panel-tag-info">{l.kind}</span>
                        </td>
                        <td><code>{short(l.target_id)}</code></td>
                        <td>{new Date(l.linked_at).toLocaleString()}</td>
                        <td>{l.note ?? ''}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </section>
          </>
        )}
      </div>

      {/* New Goal Modal */}
      {showNewModal && (
        <div
          style={{
            position: 'fixed',
            inset: 0,
            background: 'rgba(0,0,0,0.4)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            zIndex: 100,
          }}
          onClick={() => setShowNewModal(false)}
        >
          <div
            className="panel-card"
            style={{ width: 480, padding: 20 }}
            onClick={(e) => e.stopPropagation()}
          >
            <h3 style={{ marginBottom: 12 }}>New Goal</h3>
            <label style={{ display: 'block', marginBottom: 8 }}>
              <span style={{ fontSize: 'var(--font-size-sm)' }}>Title (≤120 chars)</span>
              <input
                type="text"
                value={newTitle}
                onChange={(e) => setNewTitle(e.target.value)}
                maxLength={120}
                placeholder="e.g. Ship the /goal feature"
                style={{ width: '100%', padding: 8 }}
                autoFocus
              />
            </label>
            <label style={{ display: 'block', marginBottom: 12 }}>
              <span style={{ fontSize: 'var(--font-size-sm)' }}>Statement (optional)</span>
              <textarea
                value={newStatement}
                onChange={(e) => setNewStatement(e.target.value)}
                placeholder="Describe what success looks like, the constraints, why this matters…"
                style={{ width: '100%', padding: 8, minHeight: 100, fontFamily: 'inherit' }}
              />
            </label>
            {workspacePath ? (
              <p style={{ fontSize: 'var(--font-size-xs)', color: 'var(--text-tertiary)' }}>
                Will be scoped to workspace: {workspaceLabel(workspacePath)}
              </p>
            ) : (
              <p style={{ fontSize: 'var(--font-size-xs)', color: 'var(--text-tertiary)' }}>
                Will be global (visible from mobile/watch).
              </p>
            )}
            <div style={{ display: 'flex', gap: 8, marginTop: 16, justifyContent: 'flex-end' }}>
              <button type="button" className="panel-btn" onClick={() => setShowNewModal(false)}>
                Cancel
              </button>
              <button
                type="button"
                className="panel-btn panel-btn-primary"
                onClick={createGoal}
                disabled={!newTitle.trim()}
              >
                Create
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
