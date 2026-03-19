import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface WebhookConfig {
  id: string;
  name: string;
  url: string;
  secret: string;
  events: string[];
  enabled: boolean;
  created_at: number;
}

interface WebhookLogEntry {
  id: string;
  webhook_id: string;
  webhook_name: string;
  event: string;
  status: number;
  request_body: string;
  response_body: string;
  timestamp: number;
  duration_ms: number;
}

const AVAILABLE_EVENTS = [
  'agent:complete', 'agent:error', 'file:saved', 'test:pass', 'test:fail',
  'deploy:success', 'deploy:fail', 'review:complete', 'build:success', 'build:fail',
  'commit:created', 'pr:opened', 'scan:finding',
];

export function WebhookPanel() {
  const [webhooks, setWebhooks] = useState<WebhookConfig[]>([]);
  const [logs, setLogs] = useState<WebhookLogEntry[]>([]);
  const [tab, setTab] = useState<'config' | 'logs'>('config');
  const [editing, setEditing] = useState<WebhookConfig | null>(null);
  const [expandedLog, setExpandedLog] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const wh = await invoke<WebhookConfig[]>('get_webhooks');
      setWebhooks(wh);
      const lg = await invoke<WebhookLogEntry[]>('get_webhook_logs');
      setLogs(lg);
    } catch { /* first run — no data */ }
  }, []);

  useEffect(() => { load(); }, [load]);

  const handleSave = async () => {
    if (!editing) return;
    try {
      await invoke('save_webhook', { config: editing });
      setEditing(null);
      load();
    } catch (e) {
      setError(`Failed to save: ${e}`);
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm('Delete this webhook?')) return;
    try {
      await invoke('delete_webhook', { id });
      load();
    } catch (e) {
      setError(`Failed to delete: ${e}`);
    }
  };

  const handleTest = async (id: string) => {
    try {
      const result = await invoke<{ status: number; body: string }>('test_webhook', { id });
      setError(`Test result: HTTP ${result.status}\n${result.body.slice(0, 200)}`);
      load();
    } catch (e) {
      setError(`Test failed: ${e}`);
    }
  };

  const handleReplay = async (logId: string) => {
    try {
      await invoke('replay_webhook', { logId });
      setError('Replayed successfully');
      load();
    } catch (e) {
      setError(`Replay failed: ${e}`);
    }
  };

  const newWebhook = (): WebhookConfig => ({
    id: crypto.randomUUID(),
    name: '',
    url: '',
    secret: '',
    events: [],
    enabled: true,
    created_at: Date.now(),
  });

  const toggleEvent = (event: string) => {
    if (!editing) return;
    const events = editing.events.includes(event)
      ? editing.events.filter(e => e !== event)
      : [...editing.events, event];
    setEditing({ ...editing, events });
  };

  return (
    <div style={{ padding: '12px', height: '100%', overflow: 'auto', color: 'var(--text-primary)', fontSize: 13 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 12 }}>
        <h3 style={{ margin: 0, fontSize: 15 }}>Webhook Automations</h3>
        <div style={{ display: 'flex', gap: 6 }}>
          {(['config', 'logs'] as const).map(t => (
            <button key={t} onClick={() => setTab(t)} style={{
              padding: '4px 12px', fontSize: 12, borderRadius: 4, cursor: 'pointer',
              background: tab === t ? 'var(--accent-color)' : 'var(--bg-tertiary)',
              color: tab === t ? 'var(--text-on-accent)' : 'var(--text-secondary)',
              border: '1px solid var(--border-color)',
            }}>{t === 'config' ? 'Webhooks' : 'Activity Log'}</button>
          ))}
        </div>
      </div>

      {error && <div className="panel-error"><span>{error}</span><button onClick={() => setError(null)}>✕</button></div>}

      {tab === 'config' && !editing && (
        <>
          <button onClick={() => setEditing(newWebhook())} style={{
            padding: '6px 14px', marginBottom: 12, fontSize: 12, borderRadius: 4,
            background: 'var(--accent-color)', color: 'var(--text-on-accent)', border: 'none', cursor: 'pointer',
          }}>+ Add Webhook</button>

          {webhooks.length === 0 && (
            <div style={{ color: 'var(--text-secondary)', padding: 20, textAlign: 'center' }}>
              No webhooks configured. Add one to receive notifications on events.
            </div>
          )}

          {webhooks.map(wh => (
            <div key={wh.id} style={{
              padding: '10px 12px', marginBottom: 8, borderRadius: 6,
              background: 'var(--bg-secondary)', border: '1px solid var(--border-color)',
            }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <div>
                  <span style={{ fontWeight: 600 }}>{wh.name || 'Unnamed'}</span>
                  <span style={{
                    marginLeft: 8, padding: '1px 6px', borderRadius: 3, fontSize: 10,
                    background: wh.enabled ? 'rgba(34,197,94,0.15)' : 'rgba(239,68,68,0.15)',
                    color: wh.enabled ? 'var(--success-color)' : 'var(--error-color)',
                  }}>{wh.enabled ? 'Active' : 'Disabled'}</span>
                </div>
                <div style={{ display: 'flex', gap: 4 }}>
                  <button onClick={() => handleTest(wh.id)} style={smallBtn}>Test</button>
                  <button onClick={() => setEditing(wh)} style={smallBtn}>Edit</button>
                  <button onClick={() => handleDelete(wh.id)} style={{ ...smallBtn, color: 'var(--error-color)' }}>Delete</button>
                </div>
              </div>
              <div style={{ fontSize: 11, color: 'var(--text-secondary)', marginTop: 4, fontFamily: 'var(--font-mono)' }}>
                {wh.url}
              </div>
              <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap', marginTop: 6 }}>
                {wh.events.map(ev => (
                  <span key={ev} style={{
                    padding: '1px 6px', borderRadius: 3, fontSize: 10,
                    background: 'rgba(99,102,241,0.15)', color: 'var(--accent-color)',
                  }}>{ev}</span>
                ))}
              </div>
            </div>
          ))}
        </>
      )}

      {tab === 'config' && editing && (
        <div style={{ padding: 12, background: 'var(--bg-secondary)', borderRadius: 6, border: '1px solid var(--border-color)' }}>
          <h4 style={{ margin: '0 0 12px 0', fontSize: 13 }}>{editing.name ? `Edit: ${editing.name}` : 'New Webhook'}</h4>
          <div style={{ marginBottom: 8 }}>
            <label style={labelStyle}>Name</label>
            <input value={editing.name} onChange={e => setEditing({ ...editing, name: e.target.value })}
              placeholder="My Slack Webhook" style={inputStyle} />
          </div>
          <div style={{ marginBottom: 8 }}>
            <label style={labelStyle}>URL</label>
            <input value={editing.url} onChange={e => setEditing({ ...editing, url: e.target.value })}
              placeholder="https://hooks.slack.com/services/..." style={inputStyle} />
          </div>
          <div style={{ marginBottom: 8 }}>
            <label style={labelStyle}>Secret (HMAC-SHA256)</label>
            <input value={editing.secret} onChange={e => setEditing({ ...editing, secret: e.target.value })}
              placeholder="Optional signing secret" style={inputStyle} type="password" />
          </div>
          <div style={{ marginBottom: 8 }}>
            <label style={labelStyle}>Events</label>
            <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap' }}>
              {AVAILABLE_EVENTS.map(ev => (
                <button key={ev} onClick={() => toggleEvent(ev)} style={{
                  padding: '3px 8px', fontSize: 11, borderRadius: 4, cursor: 'pointer',
                  background: editing.events.includes(ev) ? 'rgba(99,102,241,0.25)' : 'var(--bg-tertiary)',
                  color: editing.events.includes(ev) ? 'var(--accent-color)' : 'var(--text-secondary)',
                  border: `1px solid ${editing.events.includes(ev) ? 'var(--accent-color)' : 'var(--border-color)'}`,
                }}>{ev}</button>
              ))}
            </div>
          </div>
          <div style={{ marginBottom: 12 }}>
            <label style={{ ...labelStyle, display: 'flex', alignItems: 'center', gap: 6 }}>
              <input type="checkbox" checked={editing.enabled}
                onChange={e => setEditing({ ...editing, enabled: e.target.checked })} />
              Enabled
            </label>
          </div>
          <div style={{ display: 'flex', gap: 8 }}>
            <button onClick={handleSave} style={{
              padding: '6px 16px', background: 'var(--accent-color)', color: 'var(--text-on-accent)',
              border: 'none', borderRadius: 4, cursor: 'pointer', fontSize: 12,
            }}>Save</button>
            <button onClick={() => setEditing(null)} style={{
              padding: '6px 16px', background: 'var(--bg-tertiary)',
              color: 'var(--text-secondary)', border: '1px solid var(--border-color)',
              borderRadius: 4, cursor: 'pointer', fontSize: 12,
            }}>Cancel</button>
          </div>
        </div>
      )}

      {tab === 'logs' && (
        <>
          {logs.length === 0 && (
            <div style={{ color: 'var(--text-secondary)', padding: 20, textAlign: 'center' }}>
              No webhook activity yet. Events will appear here after webhooks fire.
            </div>
          )}
          {logs.map(log => (
            <div key={log.id} style={{
              padding: '8px 12px', marginBottom: 6, borderRadius: 6,
              background: 'var(--bg-secondary)', border: '1px solid var(--border-color)',
              cursor: 'pointer',
            }} onClick={() => setExpandedLog(expandedLog === log.id ? null : log.id)}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
                  <span style={{
                    padding: '1px 6px', borderRadius: 3, fontSize: 10, fontWeight: 600,
                    background: log.status < 300 ? 'rgba(34,197,94,0.15)' : 'rgba(239,68,68,0.15)',
                    color: log.status < 300 ? 'var(--success-color)' : 'var(--error-color)',
                  }}>{log.status}</span>
                  <span style={{ fontWeight: 500 }}>{log.webhook_name}</span>
                  <span style={{ color: 'var(--text-secondary)', fontSize: 11 }}>{log.event}</span>
                </div>
                <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
                  <span style={{ fontSize: 11, color: 'var(--text-secondary)' }}>{log.duration_ms}ms</span>
                  <span style={{ fontSize: 11, color: 'var(--text-secondary)' }}>
                    {new Date(log.timestamp).toLocaleTimeString()}
                  </span>
                  <button onClick={(e) => { e.stopPropagation(); handleReplay(log.id); }}
                    style={{ ...smallBtn, fontSize: 10 }}>Replay</button>
                </div>
              </div>
              {expandedLog === log.id && (
                <div style={{ marginTop: 8, fontSize: 11, fontFamily: 'var(--font-mono)' }}>
                  <div style={{ marginBottom: 4, color: 'var(--text-secondary)' }}>Request:</div>
                  <pre style={{
                    background: 'var(--bg-tertiary)', padding: 8, borderRadius: 4,
                    whiteSpace: 'pre-wrap', maxHeight: 120, overflow: 'auto', margin: '0 0 8px 0',
                  }}>{log.request_body.slice(0, 1000)}</pre>
                  <div style={{ marginBottom: 4, color: 'var(--text-secondary)' }}>Response:</div>
                  <pre style={{
                    background: 'var(--bg-tertiary)', padding: 8, borderRadius: 4,
                    whiteSpace: 'pre-wrap', maxHeight: 120, overflow: 'auto', margin: 0,
                  }}>{log.response_body.slice(0, 1000)}</pre>
                </div>
              )}
            </div>
          ))}
        </>
      )}
    </div>
  );
}

const smallBtn: React.CSSProperties = {
  padding: '2px 8px', fontSize: 11, background: 'var(--bg-tertiary)',
  color: 'var(--text-secondary)', border: '1px solid var(--border-color)',
  borderRadius: 3, cursor: 'pointer',
};

const labelStyle: React.CSSProperties = {
  display: 'block', fontSize: 11, color: 'var(--text-secondary)', marginBottom: 3,
};

const inputStyle: React.CSSProperties = {
  width: '100%', padding: '5px 8px', fontSize: 12, borderRadius: 4,
  background: 'var(--bg-tertiary)', border: '1px solid var(--border-color)',
  color: 'var(--text-primary)', outline: 'none', boxSizing: 'border-box',
};
