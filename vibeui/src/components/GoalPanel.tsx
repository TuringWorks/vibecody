import { useEffect, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Target, Plus, Play, Link2, Trash2, RefreshCw, Tag } from 'lucide-react';
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
  const [loading, setLoading] = useState(false);
  const [planning, setPlanning] = useState(false);
  const [starting, setStarting] = useState(false);
  const [showNewModal, setShowNewModal] = useState(false);
  const [newTitle, setNewTitle] = useState('');
  const [newStatement, setNewStatement] = useState('');

  const refreshList = useCallback(async () => {
    setLoading(true);
    try {
      const args: Record<string, unknown> = {};
      if (statusFilter !== 'all') args.status = statusFilter;
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
  }, [statusFilter, selectedId, toast]);

  const refreshDetail = useCallback(async (id: string) => {
    try {
      const resp = (await invoke('exec_goal_get', { id })) as GoalDetailResponse;
      setDetail(resp);
    } catch (e) {
      toast.error('Failed to load goal: ' + String(e));
      setDetail(null);
    }
  }, [toast]);

  useEffect(() => { refreshList(); }, [refreshList]);

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

        <div style={{ flex: 1, overflowY: 'auto' }}>
          {goals.length === 0 ? (
            <div className="panel-empty" style={{ padding: 16 }}>
              {loading ? 'Loading…' : 'No goals yet. Click New.'}
            </div>
          ) : (
            <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>
              {goals.map((g) => {
                const isSelected = g.id === selectedId;
                return (
                  <li
                    key={g.id}
                    onClick={() => setSelectedId(g.id)}
                    style={{
                      padding: '10px 12px',
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
                    <div style={{ fontSize: 'var(--font-size-md)' }}>{g.title}</div>
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

            {/* Tags + criteria */}
            {(detail.goal.tags.length > 0 || detail.goal.success_criteria.length > 0) && (
              <section style={{ marginBottom: 16 }}>
                {detail.goal.tags.length > 0 && (
                  <div style={{ marginBottom: 8 }}>
                    <strong>Tags:</strong>{' '}
                    {detail.goal.tags.map((t) => (
                      <span key={t} className="panel-tag" style={{ marginRight: 4 }}>
                        <Tag size={10} /> {t}
                      </span>
                    ))}
                  </div>
                )}
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
            )}

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
