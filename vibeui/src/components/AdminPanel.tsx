import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { X } from 'lucide-react';

type Role = 'admin' | 'developer' | 'viewer';

interface TeamMember {
  id: string;
  name: string;
  email: string;
  role: Role;
  api_keys: string[];
  added_at: number;
  last_active: number;
}

interface AuditEntry {
  id: string;
  timestamp: number;
  actor: string;
  action: string;
  target: string;
  details: string;
}

interface PolicyRule {
  id: string;
  resource: string;
  roles: Role[];
  action: 'allow' | 'deny';
}

const ROLE_COLORS: Record<Role, string> = {
  admin: "var(--error-color)",
  developer: "var(--info-color)",
  viewer: '#6b7280',
};

const ROLE_DESCRIPTIONS: Record<Role, string> = {
  admin: 'Full access: manage team, API keys, policies, and all tools',
  developer: 'Code access: edit files, run agents, deploy, but no team management',
  viewer: 'Read-only: view code, sessions, and dashboards',
};

export function AdminPanel() {
  const [tab, setTab] = useState<'team' | 'audit' | 'policies' | 'keys'>('team');
  const [members, setMembers] = useState<TeamMember[]>([]);
  const [audit, setAudit] = useState<AuditEntry[]>([]);
  const [policies, setPolicies] = useState<PolicyRule[]>([]);
  const [editingMember, setEditingMember] = useState<TeamMember | null>(null);
  const [filterAction, setFilterAction] = useState('');
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const m = await invoke<TeamMember[]>('get_team_members');
      setMembers(m);
    } catch { /* no data yet */ }
    try {
      const a = await invoke<AuditEntry[]>('get_audit_log', { limit: 100 });
      setAudit(a);
    } catch { /* no data yet */ }
    try {
      const p = await invoke<PolicyRule[]>('get_rbac_policies');
      setPolicies(p);
    } catch { /* no data yet */ }
  }, []);

  useEffect(() => { load(); }, [load]);

  const handleSaveMember = async () => {
    if (!editingMember) return;
    try {
      await invoke('save_team_member', { member: editingMember });
      setEditingMember(null);
      load();
    } catch (e) {
      setError(`Failed: ${e}`);
    }
  };

  const handleRemoveMember = async (id: string) => {
    if (!confirm('Remove this team member?')) return;
    try {
      await invoke('remove_team_member', { id });
      load();
    } catch (e) {
      setError(`Failed: ${e}`);
    }
  };

  const handleSavePolicy = async (policy: PolicyRule) => {
    try {
      await invoke('save_rbac_policy', { policy });
      load();
    } catch (e) {
      setError(`Failed: ${e}`);
    }
  };

  const handleDeletePolicy = async (id: string) => {
    try {
      await invoke('delete_rbac_policy', { id });
      load();
    } catch (e) {
      setError(`Failed: ${e}`);
    }
  };

  const newMember = (): TeamMember => ({
    id: crypto.randomUUID(),
    name: '',
    email: '',
    role: 'developer',
    api_keys: [],
    added_at: Date.now(),
    last_active: Date.now(),
  });

  const filteredAudit = filterAction
    ? audit.filter(a => a.action.toLowerCase().includes(filterAction.toLowerCase()))
    : audit;

  const tabs = [
    { key: 'team', label: 'Team' },
    { key: 'audit', label: 'Audit Log' },
    { key: 'policies', label: 'Policies' },
    { key: 'keys', label: 'API Keys' },
  ] as const;

  return (
    <div className="panel-container" style={{ fontSize: "var(--font-size-md)" }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 12 }}>
        <h3 style={{ margin: 0, fontSize: "var(--font-size-xl)" }}>Admin Console</h3>
        <div style={{ display: 'flex', gap: 4 }}>
          {tabs.map(t => (
            <button key={t.key} onClick={() => setTab(t.key)} className={`panel-tab ${tab === t.key ? "active" : ""}`}>
              {t.label}
            </button>
          ))}
        </div>
      </div>

      {error && <div className="panel-error" role="alert"><span>{error}</span><button onClick={() => setError(null)}><X size={12} /></button></div>}

      {/* ── Team Members ── */}
      {tab === 'team' && !editingMember && (
        <>
          <button onClick={() => setEditingMember(newMember())} className="panel-btn panel-btn-primary" style={{ marginBottom: 12 }}>+ Add Member</button>

          <div style={{ display: 'grid', gap: 8 }}>
            {members.map(m => (
              <div key={m.id} className="panel-card" style={{ padding: '8px 12px' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <div>
                    <span style={{ fontWeight: 600 }}>{m.name}</span>
                    <span style={{ marginLeft: 8, color: 'var(--text-secondary)', fontSize: "var(--font-size-sm)" }}>{m.email}</span>
                    <span style={{
                      marginLeft: 8, padding: '1px 8px', borderRadius: 3, fontSize: "var(--font-size-xs)",
                      background: `${ROLE_COLORS[m.role]}22`, color: ROLE_COLORS[m.role], fontWeight: 600,
                    }}>{m.role}</span>
                  </div>
                  <div style={{ display: 'flex', gap: 4 }}>
                    <button onClick={() => setEditingMember(m)} className="panel-btn panel-btn-secondary panel-btn-xs">Edit</button>
                    <button onClick={() => handleRemoveMember(m.id)} className="panel-btn panel-btn-danger panel-btn-xs">Remove</button>
                  </div>
                </div>
                <div style={{ fontSize: "var(--font-size-xs)", color: 'var(--text-secondary)', marginTop: 4 }}>
                  Added: {new Date(m.added_at).toLocaleDateString()} | Last active: {new Date(m.last_active).toLocaleDateString()}
                  {m.api_keys.length > 0 && ` | ${m.api_keys.length} API key(s)`}
                </div>
              </div>
            ))}
            {members.length === 0 && (
              <div className="panel-empty">No team members yet. Add members to manage access.</div>
            )}
          </div>
        </>
      )}

      {tab === 'team' && editingMember && (
        <div className="panel-card" style={{ padding: 12 }}>
          <h4 style={{ margin: '0 0 12px 0', fontSize: "var(--font-size-md)" }}>
            {editingMember.name ? `Edit: ${editingMember.name}` : 'New Member'}
          </h4>
          <div style={{ marginBottom: 8 }}>
            <label className="panel-label">Name</label>
            <input value={editingMember.name} onChange={e => setEditingMember({ ...editingMember, name: e.target.value })}
              className="panel-input panel-input-full" placeholder="Jane Developer" />
          </div>
          <div style={{ marginBottom: 8 }}>
            <label className="panel-label">Email</label>
            <input value={editingMember.email} onChange={e => setEditingMember({ ...editingMember, email: e.target.value })}
              className="panel-input panel-input-full" placeholder="jane@example.com" type="email" />
          </div>
          <div style={{ marginBottom: 12 }}>
            <label className="panel-label">Role</label>
            <div style={{ display: 'flex', gap: 8 }}>
              {(Object.keys(ROLE_COLORS) as Role[]).map(role => (
                <button key={role} onClick={() => setEditingMember({ ...editingMember, role })} style={{
                  padding: '8px 16px', fontSize: "var(--font-size-base)", borderRadius: "var(--radius-xs-plus)", cursor: 'pointer', flex: 1,
                  background: editingMember.role === role ? `${ROLE_COLORS[role]}22` : 'var(--bg-tertiary)',
                  color: editingMember.role === role ? ROLE_COLORS[role] : 'var(--text-secondary)',
                  border: `1px solid ${editingMember.role === role ? ROLE_COLORS[role] : 'var(--border-color)'}`,
                }}>
                  <div style={{ fontWeight: 600, textTransform: 'capitalize' }}>{role}</div>
                  <div style={{ fontSize: "var(--font-size-xs)", marginTop: 2 }}>{ROLE_DESCRIPTIONS[role]}</div>
                </button>
              ))}
            </div>
          </div>
          <div style={{ display: 'flex', gap: 8 }}>
            <button onClick={handleSaveMember} className="panel-btn panel-btn-primary">Save</button>
            <button onClick={() => setEditingMember(null)} className="panel-btn panel-btn-secondary">Cancel</button>
          </div>
        </div>
      )}

      {/* ── Audit Log ── */}
      {tab === 'audit' && (
        <>
          <div style={{ marginBottom: 8 }}>
            <input value={filterAction} onChange={e => setFilterAction(e.target.value)}
              placeholder="Filter by action..." className="panel-input" style={{ maxWidth: 300 }} />
          </div>
          <div style={{ display: 'grid', gap: 4 }}>
            {filteredAudit.map(entry => (
              <div key={entry.id} className="panel-card" style={{ padding: '8px 8px', fontSize: "var(--font-size-base)", display: 'flex', gap: 8, alignItems: 'center' }}>
                <span style={{ color: 'var(--text-secondary)', fontSize: "var(--font-size-xs)", minWidth: 70 }}>
                  {new Date(entry.timestamp).toLocaleTimeString()}
                </span>
                <span style={{ fontWeight: 500, minWidth: 100 }}>{entry.actor}</span>
                <span style={{
                  padding: '1px 8px', borderRadius: 3, fontSize: "var(--font-size-xs)", fontWeight: 600,
                  background: entry.action.includes('delete') || entry.action.includes('remove')
                    ? 'color-mix(in srgb, var(--accent-rose) 15%, transparent)' : 'rgba(59,130,246,0.15)',
                  color: entry.action.includes('delete') || entry.action.includes('remove')
                    ? 'var(--error-color)' : 'var(--accent-color)',
                }}>{entry.action}</span>
                <span style={{ color: 'var(--text-secondary)' }}>{entry.target}</span>
                {entry.details && (
                  <span style={{ color: 'var(--text-secondary)', fontSize: "var(--font-size-sm)" }}>{entry.details}</span>
                )}
              </div>
            ))}
            {filteredAudit.length === 0 && (
              <div className="panel-empty">No audit entries{filterAction ? ' matching filter' : ' yet'}.</div>
            )}
          </div>
        </>
      )}

      {/* ── RBAC Policies ── */}
      {tab === 'policies' && (
        <>
          <button onClick={() => handleSavePolicy({
            id: crypto.randomUUID(), resource: '*', roles: ['admin'], action: 'allow',
          })} className="panel-btn panel-btn-primary" style={{ marginBottom: 12 }}>+ Add Policy</button>

          <div style={{ display: 'grid', gap: 8 }}>
            {policies.map(p => (
              <div key={p.id} className="panel-card" style={{ padding: '8px 12px', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
                  <span style={{
                    padding: '2px 8px', borderRadius: 3, fontSize: "var(--font-size-xs)", fontWeight: 600,
                    background: p.action === 'allow' ? 'rgba(34,197,94,0.15)' : 'color-mix(in srgb, var(--accent-rose) 15%, transparent)',
                    color: p.action === 'allow' ? 'var(--success-color)' : 'var(--error-color)',
                  }}>{p.action.toUpperCase()}</span>
                  <span className="panel-mono" style={{ fontSize: "var(--font-size-base)" }}>{p.resource}</span>
                  <div style={{ display: 'flex', gap: 4 }}>
                    {p.roles.map(r => (
                      <span key={r} style={{
                        padding: '1px 4px', borderRadius: 3, fontSize: "var(--font-size-xs)",
                        background: `${ROLE_COLORS[r]}22`, color: ROLE_COLORS[r],
                      }}>{r}</span>
                    ))}
                  </div>
                </div>
                <button onClick={() => handleDeletePolicy(p.id)} className="panel-btn panel-btn-danger panel-btn-xs">Delete</button>
              </div>
            ))}
            {policies.length === 0 && (
              <div className="panel-empty">No custom policies. Default policies from VIBECLI.md and approval_policy apply.</div>
            )}
          </div>
        </>
      )}

      {/* ── API Key Management ── */}
      {tab === 'keys' && (
        <div>
          <div style={{ marginBottom: 12, color: 'var(--text-secondary)', fontSize: "var(--font-size-base)" }}>
            Manage team-wide API keys for AI providers. Keys are encrypted and stored in the admin config.
          </div>
          <div style={{ display: 'grid', gap: 8 }}>
            {members.filter(m => m.api_keys.length > 0).map(m => (
              <div key={m.id} className="panel-card" style={{ padding: '8px 12px' }}>
                <div style={{ fontWeight: 600, marginBottom: 4 }}>{m.name}</div>
                {m.api_keys.map((k, i) => (
                  <div key={i} className="panel-mono" style={{ fontSize: "var(--font-size-sm)", color: 'var(--text-secondary)' }}>
                    {k.slice(0, 8)}...{k.slice(-4)}
                  </div>
                ))}
              </div>
            ))}
            {members.filter(m => m.api_keys.length > 0).length === 0 && (
              <div className="panel-empty">No team API keys configured. Add keys to team members in the Team tab.</div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

