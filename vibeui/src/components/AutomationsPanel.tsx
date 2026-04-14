import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type TriggerSource = 'github' | 'slack' | 'linear' | 'pagerduty' | 'telegram' | 'signal' | 'whatsapp' | 'discord' | 'teams' | 'matrix' | 'twilio_sms' | 'imessage' | 'irc' | 'twitch' | 'cron' | 'filewatch' | 'webhook';

type ResolutionMode = 'auto' | 'draft' | 'approve' | 'ignore';

interface AutomationRule {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
  trigger: TriggerSource;
  events: string[];
  filter: string;
  promptTemplate: string;
  provider: string;
  maxTurns: number;
  sandbox: boolean;
  fireCount: number;
  lastFired: string | null;
  resolution_mode: ResolutionMode;
}

interface AutomationTask {
  taskId: string;
  ruleId: string;
  prompt: string;
  status: 'queued' | 'running' | 'completed' | 'failed' | 'cancelled';
  createdAt: string;
  completedAt: string | null;
  output: string | null;
}

interface AutomationStats {
  totalRules: number;
  enabledRules: number;
  totalTasks: number;
  runningTasks: number;
  completedTasks: number;
  failedTasks: number;
}

interface LogEntry {
  timestamp: string;
  message: string;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const triggerIcons: Record<TriggerSource, string> = {
  github: 'GH', slack: 'SL', linear: 'LN', pagerduty: 'PD',
  telegram: 'TG', signal: 'SG', whatsapp: 'WA', discord: 'DC',
  teams: 'MS', matrix: 'MX', twilio_sms: 'TW', imessage: 'iM',
  irc: 'IR', twitch: 'TV', cron: 'CR', filewatch: 'FW', webhook: 'WH',
};

const statusColors: Record<string, string> = {
  queued: 'var(--text-secondary)', running: 'var(--accent-color)',
  completed: 'var(--success-color)', failed: 'var(--error-color)',
  cancelled: 'var(--text-secondary)',
};

const RESOLUTION_MODE_COLORS: Record<ResolutionMode, string> = {
  auto: 'var(--accent-green)',
  draft: 'var(--accent-blue)',
  approve: 'var(--accent-gold)',
  ignore: 'var(--text-secondary)',
};

const RESOLUTION_MODE_BG: Record<ResolutionMode, string> = {
  auto: 'rgba(39,174,96,0.15)',
  draft: 'rgba(74,158,255,0.15)',
  approve: 'rgba(255,193,7,0.15)',
  ignore: 'rgba(128,128,128,0.15)',
};

const RESOLUTION_MODE_DESCRIPTIONS: Record<ResolutionMode, string> = {
  auto: 'Execute automatically',
  draft: 'Generate draft for review',
  approve: 'Route to Approvals panel',
  ignore: 'Log but take no action',
};

function ResolutionBadge({ ruleId, mode, onChange }: { ruleId: string; mode: ResolutionMode; onChange: (ruleId: string, mode: ResolutionMode) => void }) {
  const [open, setOpen] = useState(false);
  return (
    <div style={{ position: 'relative', display: 'inline-block' }}>
      <button
        onClick={() => setOpen((v) => !v)}
        title={RESOLUTION_MODE_DESCRIPTIONS[mode]}
        style={{
          padding: '1px 8px', borderRadius: "var(--radius-md)", fontSize: "var(--font-size-xs)", fontWeight: 600, cursor: 'pointer',
          background: RESOLUTION_MODE_BG[mode], color: RESOLUTION_MODE_COLORS[mode],
          border: `1px solid ${RESOLUTION_MODE_COLORS[mode]}`,
        }}
      >
        {mode}
      </button>
      {open && (
        <div style={{
          position: 'absolute', top: '110%', left: 0, zIndex: 50, minWidth: 180,
          background: 'var(--bg-secondary)', border: '1px solid var(--border-color)', borderRadius: "var(--radius-sm)",
          boxShadow: '0 4px 12px rgba(0,0,0,0.3)',
        }}>
          {(['auto', 'draft', 'approve', 'ignore'] as ResolutionMode[]).map((m) => (
            <button
              key={m}
              onClick={() => { onChange(ruleId, m); setOpen(false); }}
              style={{
                display: 'block', width: '100%', textAlign: 'left', padding: '6px 12px',
                background: m === mode ? RESOLUTION_MODE_BG[m] : 'transparent',
                color: RESOLUTION_MODE_COLORS[m], border: 'none', cursor: 'pointer', fontSize: "var(--font-size-base)",
              }}
            >
              <strong>{m}</strong> — {RESOLUTION_MODE_DESCRIPTIONS[m]}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

const AutomationsPanel: React.FC = () => {
  const [tab, setTab] = useState<'rules' | 'tasks' | 'logs'>('rules');
  const [rules, setRules] = useState<AutomationRule[]>([]);
  const [tasks, setTasks] = useState<AutomationTask[]>([]);
  const [stats, setStats] = useState<AutomationStats>({ totalRules: 0, enabledRules: 0, totalTasks: 0, runningTasks: 0, completedTasks: 0, failedTasks: 0 });
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [newTrigger, setNewTrigger] = useState<TriggerSource>('github');
  const [newName, setNewName] = useState('');
  const [newEvents, setNewEvents] = useState('');
  const [newPrompt, setNewPrompt] = useState('');
  const [newResolutionMode, setNewResolutionMode] = useState<ResolutionMode>('auto');

  useEffect(() => { loadAll(); }, []);

  async function loadAll() {
    setLoading(true);
    try {
      const [r, t, s, l] = await Promise.all([
        invoke<AutomationRule[]>('get_automation_rules').catch(() => []),
        invoke<AutomationTask[]>('get_automation_tasks').catch(() => []),
        invoke<AutomationStats>('get_automation_stats').catch(() => stats),
        invoke<LogEntry[]>('get_automation_logs').catch(() => []),
      ]);
      setRules(r);
      setTasks(t);
      setStats(s);
      setLogs(l);
    } finally {
      setLoading(false);
    }
  }

  async function handleToggle(ruleId: string) {
    try {
      const newState = await invoke<boolean>('toggle_automation_rule', { ruleId });
      setRules((prev) => prev.map((r) => r.id === ruleId ? { ...r, enabled: newState } : r));
      setStats((prev) => ({
        ...prev,
        enabledRules: prev.enabledRules + (newState ? 1 : -1),
      }));
    } catch { /* ignore */ }
  }

  async function handleDelete(ruleId: string) {
    try {
      await invoke('delete_automation_rule', { ruleId });
      setRules((prev) => prev.filter((r) => r.id !== ruleId));
      const s = await invoke<AutomationStats>('get_automation_stats').catch(() => stats);
      setStats(s);
    } catch { /* ignore */ }
  }

  async function handleCreate() {
    if (!newName.trim()) return;
    try {
      const rule = await invoke<AutomationRule>('create_automation_rule', {
        rule: {
          name: newName.trim(),
          description: '',
          enabled: true,
          trigger: newTrigger,
          events: newEvents.split(',').map((e) => e.trim()).filter(Boolean),
          filter: '',
          promptTemplate: newPrompt,
          provider: 'claude',
          maxTurns: 10,
          sandbox: false,
          resolution_mode: newResolutionMode,
        },
      });
      setRules((prev) => [...prev, rule]);
      setShowCreateModal(false);
      setNewName('');
      setNewEvents('');
      setNewPrompt('');
      setNewResolutionMode('auto');
      const s = await invoke<AutomationStats>('get_automation_stats').catch(() => stats);
      setStats(s);
    } catch { /* ignore */ }
  }

  async function handleResolutionChange(ruleId: string, resolutionMode: ResolutionMode) {
    try {
      await invoke('set_automation_resolution_mode', { ruleId, resolutionMode });
      setRules((prev) => prev.map((r) => r.id === ruleId ? { ...r, resolution_mode: resolutionMode } : r));
    } catch { /* ignore */ }
  }

  return (
    <div className="panel-container">
      <div className="panel-header">
        <h3>Event-Driven Automations</h3>
      </div>
      <div className="panel-body">
      {/* Stats bar */}
      <div style={{ display: 'flex', gap: 16, marginBottom: 16, flexWrap: 'wrap' }}>
        {[
          { label: 'Rules', value: `${stats.enabledRules}/${stats.totalRules}` },
          { label: 'Total Runs', value: stats.totalTasks },
          { label: 'Running', value: stats.runningTasks },
          { label: 'Completed', value: stats.completedTasks },
          { label: 'Failed', value: stats.failedTasks },
        ].map((s) => (
          <div key={s.label} style={{ background: 'var(--bg-secondary)', padding: '8px 16px', borderRadius: "var(--radius-sm)", textAlign: 'center' }}>
            <div style={{ fontSize: 20, fontWeight: 700, fontFamily: "var(--font-mono)", color: 'var(--accent-color)' }}>{s.value}</div>
            <div style={{ fontSize: "var(--font-size-sm)", color: 'var(--text-secondary)' }}>{s.label}</div>
          </div>
        ))}
      </div>

      {/* Tabs */}
      <div className="panel-tab-bar">
        {(['rules', 'tasks', 'logs'] as const).map((t) => (
          <button key={t} onClick={() => setTab(t)} className={`panel-tab${tab === t ? ' active' : ''}`}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
        <button onClick={() => setShowCreateModal(!showCreateModal)} className="panel-btn panel-btn-primary panel-btn-sm" style={{ marginLeft: 'auto' }}>
          + New Rule
        </button>
      </div>

      {/* Create modal */}
      {showCreateModal && (
        <div className="panel-card" style={{ marginBottom: 12 }}>
          <div style={{ fontWeight: 600, marginBottom: 8 }}>Create Automation Rule</div>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
            <label style={{ fontSize: "var(--font-size-base)" }}>
              Name
              <input type="text" value={newName} onChange={(e) => setNewName(e.target.value)} placeholder="e.g. PR Review Agent" className="panel-input panel-input-full" />
            </label>
            <label style={{ fontSize: "var(--font-size-base)" }}>
              Trigger Source
              <select value={newTrigger} onChange={(e) => setNewTrigger(e.target.value as TriggerSource)} className="panel-select panel-input-full">
                <option value="github">GitHub</option>
                <option value="slack">Slack</option>
                <option value="linear">Linear</option>
                <option value="pagerduty">PagerDuty</option>
                <option value="telegram">Telegram</option>
                <option value="signal">Signal</option>
                <option value="whatsapp">WhatsApp</option>
                <option value="discord">Discord</option>
                <option value="teams">Microsoft Teams</option>
                <option value="matrix">Matrix</option>
                <option value="twilio_sms">Twilio SMS</option>
                <option value="imessage">iMessage</option>
                <option value="irc">IRC</option>
                <option value="twitch">Twitch</option>
                <option value="cron">Cron</option>
                <option value="filewatch">File Watch</option>
                <option value="webhook">Webhook</option>
              </select>
            </label>
            <label style={{ fontSize: "var(--font-size-base)" }}>
              Resolution Mode
              <select value={newResolutionMode} onChange={(e) => setNewResolutionMode(e.target.value as ResolutionMode)} className="panel-select panel-input-full">
                <option value="auto">Auto — {RESOLUTION_MODE_DESCRIPTIONS.auto}</option>
                <option value="draft">Draft — {RESOLUTION_MODE_DESCRIPTIONS.draft}</option>
                <option value="approve">Approve — {RESOLUTION_MODE_DESCRIPTIONS.approve}</option>
                <option value="ignore">Ignore — {RESOLUTION_MODE_DESCRIPTIONS.ignore}</option>
              </select>
            </label>
            <label style={{ fontSize: "var(--font-size-base)", gridColumn: 'span 2' }}>
              Events (comma-separated)
              <input type="text" value={newEvents} onChange={(e) => setNewEvents(e.target.value)} placeholder="e.g. push, pull_request.opened" className="panel-input panel-input-full" />
            </label>
            <label style={{ fontSize: "var(--font-size-base)", gridColumn: 'span 2' }}>
              Prompt Template
              <textarea value={newPrompt} onChange={(e) => setNewPrompt(e.target.value)} placeholder="Use {{variables}} from event payload" rows={3} className="panel-input panel-textarea panel-input-full" style={{ fontFamily: 'var(--font-mono)' }} />
            </label>
          </div>
          <div style={{ display: 'flex', gap: 8, marginTop: 8 }}>
            <button onClick={handleCreate} className="panel-btn panel-btn-primary panel-btn-sm">Create</button>
            <button onClick={() => setShowCreateModal(false)} className="panel-btn panel-btn-secondary panel-btn-sm">Cancel</button>
          </div>
        </div>
      )}

      {loading && (
        <div className="panel-loading">Loading...</div>
      )}

      {/* Rules tab */}
      {!loading && tab === 'rules' && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          {rules.length === 0 && (
            <div style={{ textAlign: 'center', padding: 24, color: 'var(--text-secondary)', lineHeight: 1.7 }}>
              No automation rules yet.<br />Click <strong>+ New Rule</strong> to create one.
            </div>
          )}
          {rules.map((rule) => (
            <div key={rule.id} style={{
              background: 'var(--bg-secondary)', padding: 12, borderRadius: "var(--radius-sm-alt)",
              border: `1px solid ${rule.enabled ? 'var(--border-color)' : 'var(--text-secondary)'}`,
              opacity: rule.enabled ? 1 : 0.6,
            }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 6, flexWrap: 'wrap' }}>
                <span style={{
                  display: 'inline-block', padding: '2px 6px', borderRadius: "var(--radius-xs-plus)", fontSize: "var(--font-size-sm)",
                  fontWeight: 700, background: 'var(--accent-color)', color: 'var(--text-primary)',
                }}>{triggerIcons[rule.trigger]}</span>
                <strong>{rule.name}</strong>
                {/* Resolution badge with inline change */}
                <ResolutionBadge ruleId={rule.id} mode={rule.resolution_mode ?? 'auto'} onChange={handleResolutionChange} />
                {(rule.resolution_mode ?? 'auto') === 'approve' && (
                  <span style={{ fontSize: "var(--font-size-xs)", color: 'var(--accent-gold)' }}>→ routes to Approvals</span>
                )}
                <span style={{ fontSize: "var(--font-size-sm)", color: 'var(--text-secondary)', marginLeft: 'auto' }}>
                  {rule.fireCount} runs {rule.lastFired ? `· last ${rule.lastFired}` : ''}
                </span>
                <button onClick={() => handleToggle(rule.id)} style={{
                  padding: '2px 8px', fontSize: "var(--font-size-xs)", borderRadius: 3, cursor: 'pointer',
                  border: '1px solid var(--border-color)', background: 'none',
                  color: rule.enabled ? 'var(--success-color)' : 'var(--text-secondary)',
                }}>
                  {rule.enabled ? 'Enabled' : 'Disabled'}
                </button>
                <button onClick={() => handleDelete(rule.id)} style={{
                  padding: '2px 8px', fontSize: "var(--font-size-xs)", borderRadius: 3, cursor: 'pointer',
                  border: '1px solid var(--border-color)', background: 'none',
                  color: 'var(--error-color)',
                }}>
                  Delete
                </button>
              </div>
              {rule.description && <div style={{ fontSize: "var(--font-size-base)", color: 'var(--text-secondary)', marginBottom: 4 }}>{rule.description}</div>}
              <div style={{ fontSize: "var(--font-size-sm)", color: 'var(--text-secondary)' }}>
                Events: {rule.events.join(', ')} · Provider: {rule.provider} · Max turns: {rule.maxTurns}
                {rule.sandbox && ' · Sandbox'}
              </div>
              {rule.promptTemplate && (
                <div style={{ fontFamily: 'var(--font-mono)', fontSize: "var(--font-size-sm)", color: 'var(--text-secondary)', marginTop: 4, padding: 4, background: 'var(--bg-primary)', borderRadius: "var(--radius-xs-plus)" }}>
                  {rule.promptTemplate}
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {/* Tasks tab */}
      {!loading && tab === 'tasks' && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
          {tasks.length === 0 && (
            <div style={{ textAlign: 'center', padding: 24, color: 'var(--text-secondary)' }}>
              No tasks yet. Tasks appear here when automation rules fire.
            </div>
          )}
          {tasks.map((task) => (
            <div key={task.taskId} style={{
              background: 'var(--bg-secondary)', padding: 10, borderRadius: "var(--radius-sm)",
              borderLeft: `3px solid ${statusColors[task.status]}`,
            }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
                <span style={{
                  fontSize: "var(--font-size-sm)", padding: '1px 6px', borderRadius: 3,
                  background: statusColors[task.status], color: 'var(--text-primary)', fontWeight: 600,
                }}>{task.status}</span>
                <span style={{ fontSize: "var(--font-size-base)", fontFamily: 'var(--font-mono)', color: 'var(--text-secondary)' }}>{task.taskId}</span>
                <span style={{ fontSize: "var(--font-size-sm)", color: 'var(--text-secondary)', marginLeft: 'auto' }}>{task.createdAt}</span>
              </div>
              <div style={{ fontSize: "var(--font-size-base)", color: 'var(--text-primary)' }}>{task.prompt}</div>
              {task.output && (
                <div style={{ fontSize: "var(--font-size-sm)", color: 'var(--text-secondary)', marginTop: 4 }}>
                  Output: {task.output}
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {/* Logs tab */}
      {!loading && tab === 'logs' && (
        <div style={{ fontFamily: 'var(--font-mono)', fontSize: "var(--font-size-base)", background: 'var(--bg-secondary)', padding: 12, borderRadius: "var(--radius-sm-alt)", lineHeight: 1.8, color: 'var(--text-secondary)', minHeight: 100 }}>
          {logs.length === 0 && <div>No log entries yet. Logs appear when automation rules fire.</div>}
          {logs.map((entry, i) => (
            <div key={i}>[{entry.timestamp}] {entry.message}</div>
          ))}
        </div>
      )}
      </div>
    </div>
  );
};

export default AutomationsPanel;
