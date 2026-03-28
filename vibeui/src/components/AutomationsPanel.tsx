import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type TriggerSource = 'github' | 'slack' | 'linear' | 'pagerduty' | 'telegram' | 'signal' | 'whatsapp' | 'discord' | 'teams' | 'matrix' | 'twilio_sms' | 'imessage' | 'irc' | 'twitch' | 'cron' | 'filewatch' | 'webhook';

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
        },
      });
      setRules((prev) => [...prev, rule]);
      setShowCreateModal(false);
      setNewName('');
      setNewEvents('');
      setNewPrompt('');
      const s = await invoke<AutomationStats>('get_automation_stats').catch(() => stats);
      setStats(s);
    } catch { /* ignore */ }
  }

  return (
    <div style={{ padding: 16, color: 'var(--text-primary)', background: 'var(--bg-primary)', minHeight: '100%' }}>
      <h2 style={{ margin: '0 0 12px' }}>Event-Driven Automations</h2>

      {/* Stats bar */}
      <div style={{ display: 'flex', gap: 16, marginBottom: 16, flexWrap: 'wrap' }}>
        {[
          { label: 'Rules', value: `${stats.enabledRules}/${stats.totalRules}` },
          { label: 'Total Runs', value: stats.totalTasks },
          { label: 'Running', value: stats.runningTasks },
          { label: 'Completed', value: stats.completedTasks },
          { label: 'Failed', value: stats.failedTasks },
        ].map((s) => (
          <div key={s.label} style={{ background: 'var(--bg-secondary)', padding: '8px 16px', borderRadius: 6, textAlign: 'center' }}>
            <div style={{ fontSize: 20, fontWeight: 700, fontFamily: "var(--font-mono)", color: 'var(--accent-color)' }}>{s.value}</div>
            <div style={{ fontSize: 11, color: 'var(--text-secondary)' }}>{s.label}</div>
          </div>
        ))}
      </div>

      {/* Tabs */}
      <div style={{ display: 'flex', gap: 4, marginBottom: 12, borderBottom: '1px solid var(--border-color)' }}>
        {(['rules', 'tasks', 'logs'] as const).map((t) => (
          <button key={t} onClick={() => setTab(t)} style={{
            padding: '6px 16px', border: 'none', cursor: 'pointer', fontSize: 13,
            background: tab === t ? 'var(--accent-color)' : 'transparent',
            color: tab === t ? 'var(--text-primary)' : 'var(--text-secondary)', borderRadius: '6px 6px 0 0',
          }}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
        <button onClick={() => setShowCreateModal(!showCreateModal)} style={{
          marginLeft: 'auto', padding: '6px 14px', border: 'none', cursor: 'pointer',
          background: 'var(--success-color)', color: 'var(--text-primary)', borderRadius: 6, fontSize: 13,
        }}>
          + New Rule
        </button>
      </div>

      {/* Create modal */}
      {showCreateModal && (
        <div style={{ background: 'var(--bg-secondary)', padding: 16, borderRadius: 8, marginBottom: 12, border: '1px solid var(--border-color)' }}>
          <h3 style={{ margin: '0 0 8px' }}>Create Automation Rule</h3>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
            <label style={{ fontSize: 12 }}>
              Name
              <input type="text" value={newName} onChange={(e) => setNewName(e.target.value)} placeholder="e.g. PR Review Agent" style={{ width: '100%', padding: 6, background: 'var(--bg-primary)', color: 'var(--text-primary)', border: '1px solid var(--border-color)', borderRadius: 4 }} />
            </label>
            <label style={{ fontSize: 12 }}>
              Trigger Source
              <select value={newTrigger} onChange={(e) => setNewTrigger(e.target.value as TriggerSource)} style={{ width: '100%', padding: 6, background: 'var(--bg-primary)', color: 'var(--text-primary)', border: '1px solid var(--border-color)', borderRadius: 4 }}>
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
            <label style={{ fontSize: 12, gridColumn: 'span 2' }}>
              Events (comma-separated)
              <input type="text" value={newEvents} onChange={(e) => setNewEvents(e.target.value)} placeholder="e.g. push, pull_request.opened" style={{ width: '100%', padding: 6, background: 'var(--bg-primary)', color: 'var(--text-primary)', border: '1px solid var(--border-color)', borderRadius: 4 }} />
            </label>
            <label style={{ fontSize: 12, gridColumn: 'span 2' }}>
              Prompt Template
              <textarea value={newPrompt} onChange={(e) => setNewPrompt(e.target.value)} placeholder="Use {{variables}} from event payload" rows={3} style={{ width: '100%', padding: 6, background: 'var(--bg-primary)', color: 'var(--text-primary)', border: '1px solid var(--border-color)', borderRadius: 4, fontFamily: 'var(--font-mono)' }} />
            </label>
          </div>
          <div style={{ display: 'flex', gap: 8, marginTop: 8 }}>
            <button onClick={handleCreate} style={{ padding: '6px 14px', background: 'var(--accent-color)', color: 'var(--text-primary)', border: 'none', borderRadius: 4, cursor: 'pointer' }}>Create</button>
            <button onClick={() => setShowCreateModal(false)} style={{ padding: '6px 14px', background: 'var(--bg-primary)', color: 'var(--text-secondary)', border: '1px solid var(--border-color)', borderRadius: 4, cursor: 'pointer' }}>Cancel</button>
          </div>
        </div>
      )}

      {loading && (
        <div style={{ textAlign: 'center', padding: 24, color: 'var(--text-secondary)' }}>Loading...</div>
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
              background: 'var(--bg-secondary)', padding: 12, borderRadius: 8,
              border: `1px solid ${rule.enabled ? 'var(--border-color)' : 'var(--text-secondary)'}`,
              opacity: rule.enabled ? 1 : 0.6,
            }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 6 }}>
                <span style={{
                  display: 'inline-block', padding: '2px 6px', borderRadius: 4, fontSize: 11,
                  fontWeight: 700, background: 'var(--accent-color)', color: 'var(--text-primary)',
                }}>{triggerIcons[rule.trigger]}</span>
                <strong>{rule.name}</strong>
                <span style={{ fontSize: 11, color: 'var(--text-secondary)', marginLeft: 'auto' }}>
                  {rule.fireCount} runs {rule.lastFired ? `· last ${rule.lastFired}` : ''}
                </span>
                <button onClick={() => handleToggle(rule.id)} style={{
                  padding: '2px 8px', fontSize: 10, borderRadius: 3, cursor: 'pointer',
                  border: '1px solid var(--border-color)', background: 'none',
                  color: rule.enabled ? 'var(--success-color)' : 'var(--text-secondary)',
                }}>
                  {rule.enabled ? 'Enabled' : 'Disabled'}
                </button>
                <button onClick={() => handleDelete(rule.id)} style={{
                  padding: '2px 8px', fontSize: 10, borderRadius: 3, cursor: 'pointer',
                  border: '1px solid var(--border-color)', background: 'none',
                  color: 'var(--error-color)',
                }}>
                  Delete
                </button>
              </div>
              {rule.description && <div style={{ fontSize: 12, color: 'var(--text-secondary)', marginBottom: 4 }}>{rule.description}</div>}
              <div style={{ fontSize: 11, color: 'var(--text-secondary)' }}>
                Events: {rule.events.join(', ')} · Provider: {rule.provider} · Max turns: {rule.maxTurns}
                {rule.sandbox && ' · Sandbox'}
              </div>
              {rule.promptTemplate && (
                <div style={{ fontFamily: 'var(--font-mono)', fontSize: 11, color: 'var(--text-secondary)', marginTop: 4, padding: 4, background: 'var(--bg-primary)', borderRadius: 4 }}>
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
              background: 'var(--bg-secondary)', padding: 10, borderRadius: 6,
              borderLeft: `3px solid ${statusColors[task.status]}`,
            }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
                <span style={{
                  fontSize: 11, padding: '1px 6px', borderRadius: 3,
                  background: statusColors[task.status], color: 'var(--text-primary)', fontWeight: 600,
                }}>{task.status}</span>
                <span style={{ fontSize: 12, fontFamily: 'var(--font-mono)', color: 'var(--text-secondary)' }}>{task.taskId}</span>
                <span style={{ fontSize: 11, color: 'var(--text-secondary)', marginLeft: 'auto' }}>{task.createdAt}</span>
              </div>
              <div style={{ fontSize: 12, color: 'var(--text-primary)' }}>{task.prompt}</div>
              {task.output && (
                <div style={{ fontSize: 11, color: 'var(--text-secondary)', marginTop: 4 }}>
                  Output: {task.output}
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {/* Logs tab */}
      {!loading && tab === 'logs' && (
        <div style={{ fontFamily: 'var(--font-mono)', fontSize: 12, background: 'var(--bg-secondary)', padding: 12, borderRadius: 8, lineHeight: 1.8, color: 'var(--text-secondary)', minHeight: 100 }}>
          {logs.length === 0 && <div>No log entries yet. Logs appear when automation rules fire.</div>}
          {logs.map((entry, i) => (
            <div key={i}>[{entry.timestamp}] {entry.message}</div>
          ))}
        </div>
      )}
    </div>
  );
};

export default AutomationsPanel;
