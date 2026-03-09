import React, { useState } from 'react';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type TriggerSource = 'github' | 'slack' | 'linear' | 'pagerduty' | 'cron' | 'filewatch' | 'webhook';

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

// ---------------------------------------------------------------------------
// Demo data
// ---------------------------------------------------------------------------

const DEMO_RULES: AutomationRule[] = [
  {
    id: 'auto-1', name: 'PR Review Agent', description: 'Auto-review pull requests on open',
    enabled: true, trigger: 'github', events: ['pull_request.opened', 'pull_request.synchronize'],
    filter: 'repository == "vibecody"', promptTemplate: 'Review PR #{{number}} in {{repository}}: {{title}}',
    provider: 'claude', maxTurns: 15, sandbox: true, fireCount: 23, lastFired: '2 min ago',
  },
  {
    id: 'auto-2', name: 'Incident Triage', description: 'Triage PagerDuty critical incidents',
    enabled: true, trigger: 'pagerduty', events: ['incident.triggered'],
    filter: 'severity == "critical"', promptTemplate: 'Triage incident: {{title}}. Service: {{service}}',
    provider: 'openai', maxTurns: 10, sandbox: false, fireCount: 5, lastFired: '1 hour ago',
  },
  {
    id: 'auto-3', name: 'Slack Q&A Bot', description: 'Answer questions when mentioned in #dev',
    enabled: true, trigger: 'slack', events: ['app_mention'],
    filter: 'channel == "#dev"', promptTemplate: 'Answer: {{text}}',
    provider: 'ollama', maxTurns: 5, sandbox: false, fireCount: 142, lastFired: '5 min ago',
  },
  {
    id: 'auto-4', name: 'Linear Issue Handler', description: 'Auto-assign and plan new Linear issues',
    enabled: false, trigger: 'linear', events: ['issue.created'],
    filter: 'team_id == "ENG"', promptTemplate: 'Plan implementation for: {{title}}',
    provider: 'claude', maxTurns: 8, sandbox: true, fireCount: 0, lastFired: null,
  },
];

const DEMO_TASKS: AutomationTask[] = [
  { taskId: 'task-1', ruleId: 'auto-1', prompt: 'Review PR #47 in vibecody: Add GPU terminal', status: 'completed', createdAt: '2 min ago', completedAt: '1 min ago', output: 'Approved with 2 suggestions' },
  { taskId: 'task-2', ruleId: 'auto-2', prompt: 'Triage incident: High latency on API', status: 'running', createdAt: '30 sec ago', completedAt: null, output: null },
  { taskId: 'task-3', ruleId: 'auto-3', prompt: 'Answer: How do I configure MCP?', status: 'completed', createdAt: '5 min ago', completedAt: '4 min ago', output: 'Provided MCP setup guide' },
  { taskId: 'task-4', ruleId: 'auto-1', prompt: 'Review PR #46 in vibecody: Fix auth flow', status: 'failed', createdAt: '1 hour ago', completedAt: '58 min ago', output: 'Agent timeout after 300s' },
];

const DEMO_STATS: AutomationStats = {
  totalRules: 4, enabledRules: 3, totalTasks: 170, runningTasks: 1, completedTasks: 165, failedTasks: 4,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const triggerIcons: Record<TriggerSource, string> = {
  github: 'GH', slack: 'SL', linear: 'LN', pagerduty: 'PD',
  cron: 'CR', filewatch: 'FW', webhook: 'WH',
};

const statusColors: Record<string, string> = {
  queued: 'var(--vp-c-text-2)', running: 'var(--vp-c-brand)',
  completed: 'var(--vp-c-green-1, #10b981)', failed: 'var(--vp-c-red-1, #ef4444)',
  cancelled: 'var(--vp-c-text-3)',
};

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

const AutomationsPanel: React.FC = () => {
  const [tab, setTab] = useState<'rules' | 'tasks' | 'logs'>('rules');
  const [rules] = useState<AutomationRule[]>(DEMO_RULES);
  const [tasks] = useState<AutomationTask[]>(DEMO_TASKS);
  const [stats] = useState<AutomationStats>(DEMO_STATS);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [newTrigger, setNewTrigger] = useState<TriggerSource>('github');

  return (
    <div style={{ padding: 16, color: 'var(--vp-c-text-1)', background: 'var(--vp-c-bg)', minHeight: '100%' }}>
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
          <div key={s.label} style={{ background: 'var(--vp-c-bg-soft)', padding: '8px 16px', borderRadius: 6, textAlign: 'center' }}>
            <div style={{ fontSize: 20, fontWeight: 700, color: 'var(--vp-c-brand)' }}>{s.value}</div>
            <div style={{ fontSize: 11, color: 'var(--vp-c-text-2)' }}>{s.label}</div>
          </div>
        ))}
      </div>

      {/* Tabs */}
      <div style={{ display: 'flex', gap: 4, marginBottom: 12, borderBottom: '1px solid var(--vp-c-divider)' }}>
        {(['rules', 'tasks', 'logs'] as const).map((t) => (
          <button key={t} onClick={() => setTab(t)} style={{
            padding: '6px 16px', border: 'none', cursor: 'pointer', fontSize: 13,
            background: tab === t ? 'var(--vp-c-brand)' : 'transparent',
            color: tab === t ? '#fff' : 'var(--vp-c-text-2)', borderRadius: '6px 6px 0 0',
          }}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
        <button onClick={() => setShowCreateModal(!showCreateModal)} style={{
          marginLeft: 'auto', padding: '6px 14px', border: 'none', cursor: 'pointer',
          background: 'var(--vp-c-green-1, #10b981)', color: '#fff', borderRadius: 6, fontSize: 13,
        }}>
          + New Rule
        </button>
      </div>

      {/* Create modal */}
      {showCreateModal && (
        <div style={{ background: 'var(--vp-c-bg-soft)', padding: 16, borderRadius: 8, marginBottom: 12, border: '1px solid var(--vp-c-divider)' }}>
          <h3 style={{ margin: '0 0 8px' }}>Create Automation Rule</h3>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
            <label style={{ fontSize: 12 }}>
              Name
              <input type="text" placeholder="e.g. PR Review Agent" style={{ width: '100%', padding: 6, background: 'var(--vp-c-bg)', color: 'var(--vp-c-text-1)', border: '1px solid var(--vp-c-divider)', borderRadius: 4 }} />
            </label>
            <label style={{ fontSize: 12 }}>
              Trigger Source
              <select value={newTrigger} onChange={(e) => setNewTrigger(e.target.value as TriggerSource)} style={{ width: '100%', padding: 6, background: 'var(--vp-c-bg)', color: 'var(--vp-c-text-1)', border: '1px solid var(--vp-c-divider)', borderRadius: 4 }}>
                <option value="github">GitHub</option>
                <option value="slack">Slack</option>
                <option value="linear">Linear</option>
                <option value="pagerduty">PagerDuty</option>
                <option value="cron">Cron</option>
                <option value="filewatch">File Watch</option>
                <option value="webhook">Webhook</option>
              </select>
            </label>
            <label style={{ fontSize: 12, gridColumn: 'span 2' }}>
              Events (comma-separated)
              <input type="text" placeholder="e.g. push, pull_request.opened" style={{ width: '100%', padding: 6, background: 'var(--vp-c-bg)', color: 'var(--vp-c-text-1)', border: '1px solid var(--vp-c-divider)', borderRadius: 4 }} />
            </label>
            <label style={{ fontSize: 12, gridColumn: 'span 2' }}>
              Prompt Template
              <textarea placeholder="Use {{variables}} from event payload" rows={3} style={{ width: '100%', padding: 6, background: 'var(--vp-c-bg)', color: 'var(--vp-c-text-1)', border: '1px solid var(--vp-c-divider)', borderRadius: 4, fontFamily: 'monospace' }} />
            </label>
          </div>
          <div style={{ display: 'flex', gap: 8, marginTop: 8 }}>
            <button style={{ padding: '6px 14px', background: 'var(--vp-c-brand)', color: '#fff', border: 'none', borderRadius: 4, cursor: 'pointer' }}>Create</button>
            <button onClick={() => setShowCreateModal(false)} style={{ padding: '6px 14px', background: 'var(--vp-c-bg)', color: 'var(--vp-c-text-2)', border: '1px solid var(--vp-c-divider)', borderRadius: 4, cursor: 'pointer' }}>Cancel</button>
          </div>
        </div>
      )}

      {/* Rules tab */}
      {tab === 'rules' && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          {rules.map((rule) => (
            <div key={rule.id} style={{
              background: 'var(--vp-c-bg-soft)', padding: 12, borderRadius: 8,
              border: `1px solid ${rule.enabled ? 'var(--vp-c-divider)' : 'var(--vp-c-text-3)'}`,
              opacity: rule.enabled ? 1 : 0.6,
            }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 6 }}>
                <span style={{
                  display: 'inline-block', padding: '2px 6px', borderRadius: 4, fontSize: 11,
                  fontWeight: 700, background: 'var(--vp-c-brand)', color: '#fff',
                }}>{triggerIcons[rule.trigger]}</span>
                <strong>{rule.name}</strong>
                <span style={{ fontSize: 11, color: 'var(--vp-c-text-3)', marginLeft: 'auto' }}>
                  {rule.fireCount} runs {rule.lastFired ? `· last ${rule.lastFired}` : ''}
                </span>
              </div>
              <div style={{ fontSize: 12, color: 'var(--vp-c-text-2)', marginBottom: 4 }}>{rule.description}</div>
              <div style={{ fontSize: 11, color: 'var(--vp-c-text-3)' }}>
                Events: {rule.events.join(', ')} · Provider: {rule.provider} · Max turns: {rule.maxTurns}
                {rule.sandbox && ' · Sandbox'}
              </div>
              <div style={{ fontFamily: 'monospace', fontSize: 11, color: 'var(--vp-c-text-2)', marginTop: 4, padding: 4, background: 'var(--vp-c-bg)', borderRadius: 4 }}>
                {rule.promptTemplate}
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Tasks tab */}
      {tab === 'tasks' && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
          {tasks.map((task) => (
            <div key={task.taskId} style={{
              background: 'var(--vp-c-bg-soft)', padding: 10, borderRadius: 6,
              borderLeft: `3px solid ${statusColors[task.status]}`,
            }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
                <span style={{
                  fontSize: 11, padding: '1px 6px', borderRadius: 3,
                  background: statusColors[task.status], color: '#fff', fontWeight: 600,
                }}>{task.status}</span>
                <span style={{ fontSize: 12, fontFamily: 'monospace', color: 'var(--vp-c-text-3)' }}>{task.taskId}</span>
                <span style={{ fontSize: 11, color: 'var(--vp-c-text-3)', marginLeft: 'auto' }}>{task.createdAt}</span>
              </div>
              <div style={{ fontSize: 12, color: 'var(--vp-c-text-1)' }}>{task.prompt}</div>
              {task.output && (
                <div style={{ fontSize: 11, color: 'var(--vp-c-text-2)', marginTop: 4 }}>
                  Output: {task.output}
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {/* Logs tab */}
      {tab === 'logs' && (
        <div style={{ fontFamily: 'monospace', fontSize: 12, background: 'var(--vp-c-bg-soft)', padding: 12, borderRadius: 8, lineHeight: 1.8, color: 'var(--vp-c-text-2)' }}>
          <div>[14:32:05] Webhook received: github/pull_request.opened → matched rule auto-1</div>
          <div>[14:32:05] Spawning agent task-1 (provider: claude, sandbox: true)</div>
          <div>[14:33:12] Task task-1 completed (67s, 12 turns)</div>
          <div>[14:35:22] Webhook received: pagerduty/incident.triggered → matched rule auto-2</div>
          <div>[14:35:22] Spawning agent task-2 (provider: openai, sandbox: false)</div>
          <div>[14:35:44] Slack event: app_mention in #dev → matched rule auto-3</div>
          <div>[14:35:44] Spawning agent task-3 (provider: ollama, sandbox: false)</div>
          <div>[14:36:01] Task task-3 completed (17s, 3 turns)</div>
        </div>
      )}
    </div>
  );
};

export default AutomationsPanel;
