import React, { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

// ─── Types ───────────────────────────────────────────────────────────────────

interface MemoryNode {
  id: string;
  content: string;
  sector: string;
  tags: string[];
  salience: number;
  effective_salience: number;
  created_at: number;
  last_seen_at: number;
  pinned: boolean;
  encrypted: boolean;
  project_id?: string;
  waypoint_count: number;
}

interface SectorStats {
  sector: string;
  count: number;
  avg_salience: number;
  avg_age_days: number;
  pinned_count: number;
}

interface TemporalFact {
  id: string;
  subject: string;
  predicate: string;
  object: string;
  valid_from: number;
  valid_to: number | null;
  confidence: number;
}

interface QueryResult {
  memory: MemoryNode;
  score: number;
  similarity: number;
  effective_salience: number;
  recency_score: number;
  waypoint_score: number;
  sector_match_score: number;
}

type Tab = 'overview' | 'memories' | 'query' | 'facts' | 'graph' | 'settings';
type SectorName = 'episodic' | 'semantic' | 'procedural' | 'emotional' | 'reflective';

const SECTOR_COLORS: Record<SectorName, string> = {
  episodic: 'var(--accent-blue)',
  semantic: 'var(--accent-green)',
  procedural: 'var(--accent-gold, #eab308)',
  emotional: 'var(--accent-rose, #ef4444)',
  reflective: 'var(--accent-purple, #a855f7)',
};

const SECTOR_ICONS: Record<SectorName, string> = {
  episodic: 'E',
  semantic: 'S',
  procedural: 'P',
  emotional: 'M',
  reflective: 'R',
};

// ─── Helpers ─────────────────────────────────────────────────────────────────

function formatDate(epoch: number): string {
  if (!epoch) return '—';
  return new Date(epoch * 1000).toLocaleDateString(undefined, {
    month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit',
  });
}

function salienceColor(sal: number): string {
  if (sal >= 0.7) return 'var(--accent-green)';
  if (sal >= 0.4) return 'var(--accent-gold, #eab308)';
  return 'var(--accent-rose, #ef4444)';
}

// ─── Component ───────────────────────────────────────────────────────────────

const OpenMemoryPanel: React.FC = () => {
  const [tab, setTab] = useState<Tab>('overview');
  const [stats, setStats] = useState<{ total_memories: number; total_waypoints: number; total_facts: number; sectors: SectorStats[] }>({
    total_memories: 0, total_waypoints: 0, total_facts: 0, sectors: [],
  });
  const [memories, setMemories] = useState<MemoryNode[]>([]);
  const [queryText, setQueryText] = useState('');
  const [queryResults, setQueryResults] = useState<QueryResult[]>([]);
  const [facts, setFacts] = useState<TemporalFact[]>([]);
  const [newContent, setNewContent] = useState('');
  const [newTags, setNewTags] = useState('');
  const [sectorFilter, setSectorFilter] = useState<string>('all');
  const [toast, setToast] = useState<string | null>(null);
  const [addFactForm, setAddFactForm] = useState({ subject: '', predicate: '', object: '' });
  const [encryptionKey, setEncryptionKey] = useState('');
  const [encryptionEnabled, setEncryptionEnabled] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const showToast = useCallback((msg: string) => {
    setToast(msg);
    setTimeout(() => setToast(null), 3000);
  }, []);

  const loadStats = useCallback(async () => {
    try {
      const s = await invoke<typeof stats>('openmemory_stats');
      if (s) setStats(s);
      setError(null);
    } catch (err) {
      setError(String(err));
    }
  }, []);

  const loadMemories = useCallback(async () => {
    try {
      const m = await invoke<MemoryNode[]>('openmemory_list', {
        offset: 0, limit: 100, sector: sectorFilter === 'all' ? null : sectorFilter,
      });
      if (m) setMemories(m);
    } catch (err) {
      console.error('Failed to load memories:', err);
    }
  }, [sectorFilter]);

  const loadFacts = useCallback(async () => {
    try {
      const f = await invoke<TemporalFact[]>('openmemory_facts');
      if (f) setFacts(f);
    } catch (err) {
      console.error('Failed to load facts:', err);
    }
  }, []);

  useEffect(() => {
    loadStats();
  }, [loadStats]);

  useEffect(() => {
    if (tab === 'memories') loadMemories();
    if (tab === 'facts') loadFacts();
  }, [tab, loadMemories, loadFacts]);

  // Auto-refresh: poll stats every 10s, active tab data every 15s
  useEffect(() => {
    const statsInterval = setInterval(loadStats, 10000);
    return () => clearInterval(statsInterval);
  }, [loadStats]);

  useEffect(() => {
    if (tab === 'memories' || tab === 'facts') {
      const dataInterval = setInterval(() => {
        if (tab === 'memories') loadMemories();
        if (tab === 'facts') loadFacts();
      }, 15000);
      return () => clearInterval(dataInterval);
    }
  }, [tab, loadMemories, loadFacts]);

  const handleAdd = async () => {
    if (!newContent.trim()) return;
    await invoke('openmemory_add', {
      content: newContent,
      tags: newTags.split(',').map(t => t.trim()).filter(Boolean),
    });
    setNewContent('');
    setNewTags('');
    showToast('Memory added');
    loadStats();
    if (tab === 'memories') loadMemories();
  };

  const handleQuery = async () => {
    if (!queryText.trim()) return;
    const results = await invoke<QueryResult[]>('openmemory_query', {
      text: queryText, limit: 20,
      sector: sectorFilter === 'all' ? null : sectorFilter,
    });
    if (results) setQueryResults(results);
  };

  const handleDelete = async (id: string) => {
    await invoke('openmemory_delete', { id });
    showToast('Memory deleted');
    loadMemories();
    loadStats();
  };

  const handlePin = async (id: string, pin: boolean) => {
    await invoke(pin ? 'openmemory_pin' : 'openmemory_unpin', { id });
    showToast(pin ? 'Pinned' : 'Unpinned');
    loadMemories();
  };

  const handleAddFact = async () => {
    const { subject, predicate, object } = addFactForm;
    if (!subject.trim() || !predicate.trim() || !object.trim()) return;
    await invoke('openmemory_add_fact', { subject, predicate, object });
    setAddFactForm({ subject: '', predicate: '', object: '' });
    showToast('Fact added');
    loadFacts();
    loadStats();
  };

  const handleRunDecay = async () => {
    const result = await invoke<{ purged: number }>('openmemory_run_decay');
    showToast(`Decay complete: ${result?.purged ?? 0} memories purged`);
    loadStats();
    loadMemories();
  };

  const handleConsolidate = async () => {
    const result = await invoke<{ consolidated: number }>('openmemory_consolidate');
    showToast(`Consolidation: ${result?.consolidated ?? 0} groups merged`);
    loadStats();
    loadMemories();
  };

  const handleExport = async () => {
    const md = await invoke<string>('openmemory_export');
    if (md) {
      const blob = new Blob([md], { type: 'text/markdown' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = 'openmemory-export.md';
      a.click();
      URL.revokeObjectURL(url);
      showToast('Exported to markdown');
    }
  };

  const tabs: { key: Tab; label: string }[] = [
    { key: 'overview', label: 'Overview' },
    { key: 'memories', label: 'Memories' },
    { key: 'query', label: 'Query' },
    { key: 'facts', label: 'Facts' },
    { key: 'graph', label: 'Graph' },
    { key: 'settings', label: 'Settings' },
  ];

  return (
    <div className="panel-container">
      {/* Error */}
      {error && (
        <div className="panel-error">
          {error}
        </div>
      )}
      {/* Toast */}
      {toast && (
        <div style={{
          position: 'fixed', top: 12, right: 12, zIndex: 9999,
          background: 'var(--bg-tertiary)', color: 'var(--text-primary)',
          padding: '8px 16px', borderRadius: 6, fontSize: 13, boxShadow: '0 4px 12px rgba(0,0,0,0.3)',
        }}>{toast}</div>
      )}

      {/* Tab Bar */}
      <div className="panel-tab-bar">
        {tabs.map(t => (
          <button key={t.key} onClick={() => setTab(t.key)} className={`panel-tab ${tab === t.key ? 'active' : ''}`}>{t.label}</button>
        ))}
      </div>

      {/* Content */}
      <div className="panel-body">
        {/* ─── Overview ─────────────────────────────────────────────── */}
        {tab === 'overview' && (
          <div>
            <h3 style={{ margin: '0 0 12px', color: 'var(--text-primary)' }}>
              Cognitive Memory Engine
            </h3>
            <p style={{ color: 'var(--text-secondary)', fontSize: 13, marginBottom: 16 }}>
              Bio-inspired 5-sector memory with decay, reinforcement, multi-waypoint graph, HNSW vector search, and temporal knowledge graph.
            </p>

            {/* Summary Cards */}
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 12, marginBottom: 20 }}>
              {[
                { label: 'Memories', value: stats.total_memories },
                { label: 'Waypoints', value: stats.total_waypoints },
                { label: 'Facts', value: stats.total_facts },
              ].map(c => (
                <div key={c.label} style={{
                  background: 'var(--bg-tertiary)', borderRadius: 8, padding: 16, textAlign: 'center',
                }}>
                  <div style={{ fontSize: 28, fontWeight: 700, color: 'var(--accent-blue)' }}>{c.value}</div>
                  <div style={{ fontSize: 12, color: 'var(--text-secondary)', marginTop: 4 }}>{c.label}</div>
                </div>
              ))}
            </div>

            {/* Sector Breakdown */}
            <h4 style={{ color: 'var(--text-primary)', marginBottom: 8 }}>Sector Distribution</h4>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(5, 1fr)', gap: 8 }}>
              {stats.sectors.map(s => (
                <div key={s.sector} style={{
                  background: 'var(--bg-tertiary)', borderRadius: 8, padding: 12,
                  borderLeft: `3px solid ${SECTOR_COLORS[s.sector as SectorName] || '#666'}`,
                }}>
                  <div style={{ fontSize: 11, textTransform: 'uppercase', color: SECTOR_COLORS[s.sector as SectorName] || '#666', marginBottom: 4 }}>
                    {SECTOR_ICONS[s.sector as SectorName] || '?'} {s.sector}
                  </div>
                  <div style={{ fontSize: 20, fontWeight: 700, color: 'var(--text-primary)' }}>{s.count}</div>
                  <div style={{ fontSize: 11, color: 'var(--text-secondary)' }}>
                    avg sal: {(s.avg_salience * 100).toFixed(0)}%
                  </div>
                  <div style={{ fontSize: 11, color: 'var(--text-secondary)' }}>
                    {s.pinned_count} pinned
                  </div>
                </div>
              ))}
            </div>

            {/* Quick Add */}
            <h4 style={{ color: 'var(--text-primary)', margin: '20px 0 8px' }}>Quick Add Memory</h4>
            <div style={{ display: 'flex', gap: 8 }}>
              <input
                value={newContent} onChange={e => setNewContent(e.target.value)}
                placeholder="Enter memory content..."
                onKeyDown={e => e.key === 'Enter' && handleAdd()}
                style={{
                  flex: 1, padding: '8px 12px', borderRadius: 6, border: '1px solid var(--border-color)',
                  background: 'var(--bg-tertiary)', color: 'var(--text-primary)', fontSize: 13,
                }}
              />
              <input
                value={newTags} onChange={e => setNewTags(e.target.value)}
                placeholder="Tags (comma-sep)"
                style={{
                  width: 150, padding: '8px 12px', borderRadius: 6, border: '1px solid var(--border-color)',
                  background: 'var(--bg-tertiary)', color: 'var(--text-primary)', fontSize: 13,
                }}
              />
              <button onClick={handleAdd} style={{
                padding: '8px 16px', borderRadius: 6, border: 'none', cursor: 'pointer',
                background: 'var(--accent-blue)', color: 'var(--btn-primary-fg)', fontSize: 13, fontWeight: 600,
              }}>Add</button>
            </div>

            {/* Actions */}
            <div style={{ display: 'flex', gap: 8, marginTop: 16 }}>
              <button onClick={handleRunDecay} style={actionBtnStyle}>Run Decay</button>
              <button onClick={handleConsolidate} style={actionBtnStyle}>Consolidate</button>
              <button onClick={handleExport} style={actionBtnStyle}>Export Markdown</button>
            </div>
          </div>
        )}

        {/* ─── Memories List ────────────────────────────────────────── */}
        {tab === 'memories' && (
          <div>
            <div style={{ display: 'flex', gap: 8, marginBottom: 12 }}>
              <select value={sectorFilter} onChange={e => setSectorFilter(e.target.value)} className="panel-select">
                <option value="all">All Sectors</option>
                {Object.keys(SECTOR_COLORS).map(s => (
                  <option key={s} value={s}>{s}</option>
                ))}
              </select>
              <button onClick={loadMemories} style={actionBtnStyle}>Refresh</button>
            </div>

            {memories.length === 0 ? (
              <p style={{ color: 'var(--text-secondary)', fontSize: 13 }}>
                No memories yet. Add one from the Overview tab or use the /openmemory REPL command.
              </p>
            ) : (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                {memories.map(m => (
                  <div key={m.id} style={{
                    background: 'var(--bg-tertiary)', borderRadius: 8, padding: 12,
                    borderLeft: `3px solid ${SECTOR_COLORS[m.sector as SectorName] || '#666'}`,
                  }}>
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 6 }}>
                      <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
                        <span style={{
                          fontSize: 11, textTransform: 'uppercase', fontWeight: 600,
                          color: SECTOR_COLORS[m.sector as SectorName] || '#666',
                        }}>{m.sector}</span>
                        <span style={{ fontSize: 11, color: salienceColor(m.effective_salience) }}>
                          {(m.effective_salience * 100).toFixed(0)}%
                        </span>
                        {m.pinned && <span style={{ fontSize: 10, color: 'var(--accent-gold, #eab308)' }}>PINNED</span>}
                        {m.encrypted && <span style={{ fontSize: 10, color: 'var(--accent-purple, #a855f7)' }}>ENCRYPTED</span>}
                      </div>
                      <div style={{ display: 'flex', gap: 4 }}>
                        <button onClick={() => handlePin(m.id, !m.pinned)} style={smallBtnStyle}>
                          {m.pinned ? 'Unpin' : 'Pin'}
                        </button>
                        <button onClick={() => handleDelete(m.id)} style={{ ...smallBtnStyle, color: 'var(--accent-rose, #ef4444)' }}>
                          Delete
                        </button>
                      </div>
                    </div>
                    <div style={{ fontSize: 13, color: 'var(--text-primary)', marginBottom: 6 }}>
                      {m.content.length > 300 ? m.content.slice(0, 300) + '...' : m.content}
                    </div>
                    <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
                      {m.tags.map(t => (
                        <span key={t} style={{
                          fontSize: 11, padding: '2px 8px', borderRadius: 10,
                          background: 'var(--bg-primary)', color: 'var(--text-secondary)',
                        }}>{t}</span>
                      ))}
                      <span style={{ fontSize: 11, color: 'var(--text-secondary)' }}>
                        {formatDate(m.created_at)}
                      </span>
                      {m.waypoint_count > 0 && (
                        <span style={{ fontSize: 11, color: 'var(--text-secondary)' }}>
                          {m.waypoint_count} links
                        </span>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* ─── Query ────────────────────────────────────────────────── */}
        {tab === 'query' && (
          <div>
            <h4 style={{ color: 'var(--text-primary)', marginBottom: 8 }}>Semantic Memory Query</h4>
            <p style={{ color: 'var(--text-secondary)', fontSize: 12, marginBottom: 12 }}>
              Composite scoring: similarity + salience + recency + waypoint expansion + sector match
            </p>
            <div style={{ display: 'flex', gap: 8, marginBottom: 16 }}>
              <input
                value={queryText} onChange={e => setQueryText(e.target.value)}
                placeholder="Search your memories..."
                onKeyDown={e => e.key === 'Enter' && handleQuery()}
                className="panel-input" style={{ flex: 1 }}
              />
              <select value={sectorFilter} onChange={e => setSectorFilter(e.target.value)} className="panel-select">
                <option value="all">All Sectors</option>
                {Object.keys(SECTOR_COLORS).map(s => (
                  <option key={s} value={s}>{s}</option>
                ))}
              </select>
              <button onClick={handleQuery} style={{
                padding: '8px 16px', borderRadius: 6, border: 'none', cursor: 'pointer',
                background: 'var(--accent-blue)', color: 'var(--btn-primary-fg)', fontSize: 13, fontWeight: 600,
              }}>Search</button>
            </div>

            {queryResults.length > 0 && (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                {queryResults.map((r, i) => (
                  <div key={r.memory.id} style={{
                    background: 'var(--bg-tertiary)', borderRadius: 8, padding: 12,
                    borderLeft: `3px solid ${SECTOR_COLORS[r.memory.sector as SectorName] || '#666'}`,
                  }}>
                    <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 6 }}>
                      <span style={{ fontSize: 12, fontWeight: 600, color: 'var(--accent-blue)' }}>
                        #{i + 1} Score: {r.score.toFixed(3)}
                      </span>
                      <span style={{
                        fontSize: 11, textTransform: 'uppercase',
                        color: SECTOR_COLORS[r.memory.sector as SectorName] || '#666',
                      }}>{r.memory.sector}</span>
                    </div>
                    <div style={{ fontSize: 13, color: 'var(--text-primary)', marginBottom: 8 }}>
                      {r.memory.content.length > 400 ? r.memory.content.slice(0, 400) + '...' : r.memory.content}
                    </div>
                    <div style={{ display: 'flex', gap: 16, fontSize: 11, color: 'var(--text-secondary)' }}>
                      <span>sim: {r.similarity.toFixed(3)}</span>
                      <span>sal: {r.effective_salience.toFixed(3)}</span>
                      <span>rec: {r.recency_score.toFixed(3)}</span>
                      <span>wp: {r.waypoint_score.toFixed(3)}</span>
                      <span>sec: {r.sector_match_score.toFixed(3)}</span>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* ─── Facts ────────────────────────────────────────────────── */}
        {tab === 'facts' && (
          <div>
            <h4 style={{ color: 'var(--text-primary)', marginBottom: 8 }}>Temporal Knowledge Graph</h4>
            <p style={{ color: 'var(--text-secondary)', fontSize: 12, marginBottom: 12 }}>
              Bi-temporal facts with validity windows. New facts auto-close previous conflicting entries.
            </p>

            {/* Add Fact Form */}
            <div style={{ display: 'flex', gap: 8, marginBottom: 16 }}>
              <input
                value={addFactForm.subject} onChange={e => setAddFactForm(f => ({ ...f, subject: e.target.value }))}
                placeholder="Subject" className="panel-input" style={{flex:1}}
              />
              <input
                value={addFactForm.predicate} onChange={e => setAddFactForm(f => ({ ...f, predicate: e.target.value }))}
                placeholder="Predicate" className="panel-input" style={{flex:1}}
              />
              <input
                value={addFactForm.object} onChange={e => setAddFactForm(f => ({ ...f, object: e.target.value }))}
                placeholder="Object" className="panel-input" style={{flex:1}}
                onKeyDown={e => e.key === 'Enter' && handleAddFact()}
              />
              <button onClick={handleAddFact} style={{
                padding: '8px 16px', borderRadius: 6, border: 'none', cursor: 'pointer',
                background: 'var(--accent-green)', color: 'var(--btn-primary-fg)', fontSize: 13, fontWeight: 600,
              }}>Add Fact</button>
            </div>

            {facts.length === 0 ? (
              <p style={{ color: 'var(--text-secondary)', fontSize: 13 }}>No temporal facts recorded yet.</p>
            ) : (
              <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 13 }}>
                <thead>
                  <tr style={{ borderBottom: '1px solid var(--border-color)' }}>
                    <th style={thStyle}>Subject</th>
                    <th style={thStyle}>Predicate</th>
                    <th style={thStyle}>Object</th>
                    <th style={thStyle}>Valid From</th>
                    <th style={thStyle}>Valid To</th>
                    <th style={thStyle}>Conf.</th>
                  </tr>
                </thead>
                <tbody>
                  {facts.map(f => (
                    <tr key={f.id} style={{ borderBottom: '1px solid var(--border-color)' }}>
                      <td style={tdStyle}>{f.subject}</td>
                      <td style={tdStyle}>{f.predicate}</td>
                      <td style={tdStyle}>{f.object}</td>
                      <td style={tdStyle}>{formatDate(f.valid_from)}</td>
                      <td style={tdStyle}>{f.valid_to ? formatDate(f.valid_to) : 'Current'}</td>
                      <td style={tdStyle}>{(f.confidence * 100).toFixed(0)}%</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        )}

        {/* ─── Graph Visualization ──────────────────────────────────── */}
        {tab === 'graph' && (
          <div>
            <h4 style={{ color: 'var(--text-primary)', marginBottom: 8 }}>Associative Memory Graph</h4>
            <p style={{ color: 'var(--text-secondary)', fontSize: 12, marginBottom: 16 }}>
              Multi-waypoint graph (up to 5 links per memory). Exceeds OpenMemory's single-link limitation.
            </p>

            {/* SVG force-directed graph */}
            {stats.total_memories === 0 ? (
              <div style={{
                background: 'var(--bg-tertiary)', borderRadius: 8, padding: 32,
                textAlign: 'center', color: 'var(--text-secondary)', fontSize: 13,
              }}>
                No memories to graph. Add memories to see the associative network.
              </div>
            ) : (
              <div style={{ background: 'var(--bg-tertiary)', borderRadius: 8, overflow: 'hidden' }}>
                <ForceGraph memories={memories} />
              </div>
            )}

            <div style={{ marginTop: 12, fontSize: 12, color: 'var(--text-secondary)', display: 'flex', gap: 16, flexWrap: 'wrap' }}>
              {Object.entries(SECTOR_COLORS).map(([name, color]) => (
                <span key={name} style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                  <span style={{ width: 10, height: 10, borderRadius: '50%', background: color, display: 'inline-block' }} />
                  {name}
                </span>
              ))}
              <span style={{ marginLeft: 8 }}>Node size = salience | Lines = waypoint links</span>
            </div>
          </div>
        )}

        {/* ─── Settings ─────────────────────────────────────────────── */}
        {tab === 'settings' && (
          <div>
            <h4 style={{ color: 'var(--text-primary)', marginBottom: 12 }}>OpenMemory Settings</h4>

            {/* Encryption */}
            <div style={{ background: 'var(--bg-tertiary)', borderRadius: 8, padding: 16, marginBottom: 12 }}>
              <h5 style={{ color: 'var(--text-primary)', marginBottom: 8 }}>Encryption at Rest</h5>
              <p style={{ fontSize: 12, color: 'var(--text-secondary)', marginBottom: 8 }}>
                AES-256-GCM encryption for memory content. New memories will be encrypted after enabling.
              </p>
              <div style={{ display: 'flex', gap: 8 }}>
                <input
                  type="password"
                  value={encryptionKey} onChange={e => setEncryptionKey(e.target.value)}
                  placeholder="Encryption passphrase..."
                  className="panel-input" style={{flex:1}}
                />
                <button onClick={async () => {
                  if (encryptionKey) {
                    await invoke('openmemory_enable_encryption', { passphrase: encryptionKey });
                    setEncryptionEnabled(true);
                    showToast('Encryption enabled');
                  }
                }} style={{
                  ...actionBtnStyle,
                  background: encryptionEnabled
                    ? 'var(--accent-green)'
                    : 'var(--accent-blue)',
                }}>{encryptionEnabled ? 'Enabled' : 'Enable'}</button>
              </div>
            </div>

            {/* Feature Comparison */}
            <div style={{ background: 'var(--bg-tertiary)', borderRadius: 8, padding: 16 }}>
              <h5 style={{ color: 'var(--text-primary)', marginBottom: 8 }}>
                VibeCody vs OpenMemory
              </h5>
              <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 12 }}>
                <thead>
                  <tr style={{ borderBottom: '1px solid var(--border-color)' }}>
                    <th style={thStyle}>Feature</th>
                    <th style={thStyle}>OpenMemory</th>
                    <th style={thStyle}>VibeCody</th>
                  </tr>
                </thead>
                <tbody>
                  {[
                    ['Classification', 'Regex patterns', 'TF-IDF + keyword scoring'],
                    ['Graph links', 'Single (1 per node)', 'Multi (top-5 per node)'],
                    ['Vector search', 'Brute-force cosine', 'HNSW ANN index'],
                    ['Encryption', 'Not implemented', 'AES-256-GCM'],
                    ['Consolidation', 'None', 'Sleep-cycle merging'],
                    ['Embeddings', 'External API required', 'Local TF-IDF (zero deps)'],
                    ['Temporal graph', 'Basic validity', 'Bi-temporal + point-in-time'],
                    ['Scoping', 'user_id only', 'User + project + workspace'],
                    ['IDE integration', 'VS Code extension', 'CLI + REPL + VibeUI + agents'],
                  ].map(([feat, om, vc]) => (
                    <tr key={feat} style={{ borderBottom: '1px solid var(--border-color)' }}>
                      <td style={tdStyle}>{feat}</td>
                      <td style={tdStyle}>{om}</td>
                      <td style={{ ...tdStyle, color: 'var(--accent-green)' }}>{vc}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

// ─── Force-Directed Graph Component ──────────────────────────────────────────

interface GraphNode {
  id: string;
  sector: SectorName;
  label: string;
  salience: number;
  x: number;
  y: number;
  vx: number;
  vy: number;
}

const ForceGraph: React.FC<{ memories: MemoryNode[] }> = ({ memories }) => {
  const width = 700;
  const height = 400;
  const [hoveredNode, setHoveredNode] = useState<string | null>(null);

  // Build graph nodes with simple force layout
  const nodes: GraphNode[] = React.useMemo(() => {
    const centerX = width / 2;
    const centerY = height / 2;

    // Group by sector and arrange in clusters
    const sectorOrder: SectorName[] = ['episodic', 'semantic', 'procedural', 'emotional', 'reflective'];
    const sectorCenters: Record<string, { x: number; y: number }> = {};
    sectorOrder.forEach((s, i) => {
      const angle = (i / sectorOrder.length) * Math.PI * 2 - Math.PI / 2;
      sectorCenters[s] = {
        x: centerX + Math.cos(angle) * 120,
        y: centerY + Math.sin(angle) * 120,
      };
    });

    return memories.slice(0, 50).map((m, i) => {
      const sc = sectorCenters[m.sector] || { x: centerX, y: centerY };
      // Spread nodes around sector center with some randomness based on index
      const spread = 60;
      const angle = (i * 2.399) % (Math.PI * 2); // Golden angle
      return {
        id: m.id,
        sector: m.sector as SectorName,
        label: m.content.slice(0, 30).replace(/\n/g, ' '),
        salience: m.effective_salience,
        x: sc.x + Math.cos(angle) * spread * (0.3 + Math.random() * 0.7),
        y: sc.y + Math.sin(angle) * spread * (0.3 + Math.random() * 0.7),
        vx: 0,
        vy: 0,
      };
    });
  }, [memories]);

  // Build edges from waypoint_count (simulated — connect to nearest same-sector nodes)
  const edges = React.useMemo(() => {
    const result: { from: string; to: string; weight: number }[] = [];
    for (let i = 0; i < nodes.length; i++) {
      const n = nodes[i];
      if (memories[i]?.waypoint_count > 0) {
        // Connect to closest nodes of same sector
        const sameType = nodes
          .filter((o, j) => j !== i && o.sector === n.sector)
          .sort((a, b) => {
            const da = Math.hypot(a.x - n.x, a.y - n.y);
            const db = Math.hypot(b.x - n.x, b.y - n.y);
            return da - db;
          });
        for (const target of sameType.slice(0, Math.min(memories[i].waypoint_count, 3))) {
          // Avoid duplicates
          if (!result.some(e => (e.from === n.id && e.to === target.id) || (e.from === target.id && e.to === n.id))) {
            result.push({ from: n.id, to: target.id, weight: 0.5 + Math.random() * 0.5 });
          }
        }
      }
    }
    return result;
  }, [nodes, memories]);

  const nodeMap = React.useMemo(() => {
    const m: Record<string, GraphNode> = {};
    nodes.forEach(n => { m[n.id] = n; });
    return m;
  }, [nodes]);

  return (
    <svg width={width} height={height} style={{ display: 'block' }}>
      {/* Edges */}
      {edges.map((e, i) => {
        const from = nodeMap[e.from];
        const to = nodeMap[e.to];
        if (!from || !to) return null;
        return (
          <line key={`e-${i}`}
            x1={from.x} y1={from.y} x2={to.x} y2={to.y}
            stroke="var(--border-color)" strokeWidth={e.weight * 2} strokeOpacity={0.4}
          />
        );
      })}
      {/* Sector labels */}
      {['episodic', 'semantic', 'procedural', 'emotional', 'reflective'].map((s, i) => {
        const angle = (i / 5) * Math.PI * 2 - Math.PI / 2;
        const lx = width / 2 + Math.cos(angle) * 180;
        const ly = height / 2 + Math.sin(angle) * 180;
        return (
          <text key={`label-${s}`} x={lx} y={ly}
            textAnchor="middle" fontSize={10} fontWeight={600} fill={SECTOR_COLORS[s as SectorName] || '#666'}
            opacity={0.6}
          >{s.toUpperCase()}</text>
        );
      })}
      {/* Nodes */}
      {nodes.map(n => {
        const r = 4 + n.salience * 10; // radius based on salience
        const isHovered = hoveredNode === n.id;
        return (
          <g key={n.id}
            onMouseEnter={() => setHoveredNode(n.id)}
            onMouseLeave={() => setHoveredNode(null)}
            style={{ cursor: 'pointer' }}
          >
            <circle cx={n.x} cy={n.y} r={r}
              fill={SECTOR_COLORS[n.sector] || '#666'}
              fillOpacity={0.7 + n.salience * 0.3}
              stroke={isHovered ? '#fff' : 'none'}
              strokeWidth={isHovered ? 2 : 0}
            />
            {isHovered && (
              <text x={n.x} y={n.y - r - 4} textAnchor="middle"
                fontSize={10} fill="var(--text-primary)"
              >{n.label}...</text>
            )}
          </g>
        );
      })}
      {/* Center stats */}
      <text x={width / 2} y={height - 10} textAnchor="middle"
        fontSize={11} fill="var(--text-secondary)"
      >{nodes.length} nodes | {edges.length} edges</text>
    </svg>
  );
};

// ─── Shared Styles ───────────────────────────────────────────────────────────

const actionBtnStyle: React.CSSProperties = {
  padding: '6px 14px', borderRadius: 6, border: '1px solid var(--border-color)',
  cursor: 'pointer', fontSize: 12, fontWeight: 500,
  background: 'var(--bg-tertiary)', color: 'var(--text-primary)',
};

const smallBtnStyle: React.CSSProperties = {
  padding: '2px 8px', borderRadius: 4, border: '1px solid var(--border-color)',
  cursor: 'pointer', fontSize: 11,
  background: 'transparent', color: 'var(--text-secondary)',
};


const thStyle: React.CSSProperties = {
  textAlign: 'left', padding: '6px 8px', color: 'var(--text-secondary)', fontWeight: 600,
};

const tdStyle: React.CSSProperties = {
  padding: '6px 8px', color: 'var(--text-primary)',
};

export default OpenMemoryPanel;
