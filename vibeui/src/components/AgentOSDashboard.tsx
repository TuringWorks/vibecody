import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Bot, Cpu, GitBranch,
  RefreshCw, Zap, BarChart3,
  Loader2,
} from "lucide-react";

/* ── Types ────────────────────────────────────────────────────────────────── */

interface PoolStats {
  total: number;
  running: number;
  queued: number;
  paused: number;
  completed: number;
  failed: number;
  cancelled: number;
  total_tokens: number;
}

interface SpawnedAgent {
  id: string;
  task: string;
  status: string;
  priority: string;
  isolation_mode: string;
  progress?: { percent: number; tokens_used: number; steps_done: number };
  created_at?: string;
}

interface SubAgent {
  id: string;
  role: string;
  status: string;
  task?: string;
  tokens_used?: number;
}

interface HostedAgent {
  id: string;
  name: string;
  command: string;
  status: string;
}

interface BranchAgent {
  id: string;
  branch: string;
  task: string;
  status: string;
  pr_url?: string;
}

interface ModeStats {
  modeId: string;
  invocations: number;
  avgTokens: number;
  lastUsed: string;
}

/* ── Styles ───────────────────────────────────────────────────────────────── */

const S = {
  root: {
    padding: 16,
    height: "100%",
    overflow: "auto",
    fontFamily: "var(--font-family, system-ui, sans-serif)",
    color: "var(--text-primary, #e0e0e0)",
    background: "var(--bg-primary, #1e1e1e)",
  } as React.CSSProperties,
  header: {
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    marginBottom: 16,
  } as React.CSSProperties,
  title: {
    display: "flex",
    alignItems: "center",
    gap: 8,
    fontSize: 16,
    fontWeight: 600,
  } as React.CSSProperties,
  refreshBtn: {
    background: "var(--bg-tertiary, #333)",
    border: "1px solid var(--border-primary, #444)",
    borderRadius: 6,
    color: "var(--text-primary, #e0e0e0)",
    padding: "4px 10px",
    cursor: "pointer",
    display: "flex",
    alignItems: "center",
    gap: 4,
    fontSize: 12,
  } as React.CSSProperties,
  grid: {
    display: "grid",
    gridTemplateColumns: "repeat(auto-fit, minmax(130px, 1fr))",
    gap: 10,
    marginBottom: 20,
  } as React.CSSProperties,
  statCard: {
    background: "var(--bg-secondary, #252525)",
    border: "1px solid var(--border-primary, #333)",
    borderRadius: 8,
    padding: "12px 14px",
    display: "flex",
    flexDirection: "column" as const,
    gap: 4,
  } as React.CSSProperties,
  statLabel: {
    fontSize: 11,
    color: "var(--text-secondary, #999)",
    textTransform: "uppercase" as const,
    letterSpacing: "0.5px",
  } as React.CSSProperties,
  statValue: {
    fontSize: 22,
    fontWeight: 700,
    fontVariantNumeric: "tabular-nums",
  } as React.CSSProperties,
  section: {
    marginBottom: 20,
  } as React.CSSProperties,
  sectionTitle: {
    fontSize: 13,
    fontWeight: 600,
    marginBottom: 8,
    display: "flex",
    alignItems: "center",
    gap: 6,
    color: "var(--text-primary, #e0e0e0)",
  } as React.CSSProperties,
  table: {
    width: "100%",
    borderCollapse: "collapse" as const,
    fontSize: 12,
  } as React.CSSProperties,
  th: {
    textAlign: "left" as const,
    padding: "6px 8px",
    borderBottom: "1px solid var(--border-primary, #333)",
    color: "var(--text-secondary, #999)",
    fontWeight: 500,
    fontSize: 11,
    textTransform: "uppercase" as const,
  } as React.CSSProperties,
  td: {
    padding: "6px 8px",
    borderBottom: "1px solid var(--border-subtle, #2a2a2a)",
    verticalAlign: "middle" as const,
  } as React.CSSProperties,
  badge: (color: string) => ({
    display: "inline-flex",
    alignItems: "center",
    gap: 4,
    padding: "2px 8px",
    borderRadius: 10,
    fontSize: 11,
    fontWeight: 500,
    background: color + "20",
    color,
  }) as React.CSSProperties,
  emptyRow: {
    padding: "12px 8px",
    color: "var(--text-secondary, #666)",
    fontSize: 12,
    fontStyle: "italic" as const,
  } as React.CSSProperties,
  modeBar: {
    display: "flex",
    gap: 10,
    flexWrap: "wrap" as const,
  } as React.CSSProperties,
  modeCard: {
    background: "var(--bg-secondary, #252525)",
    border: "1px solid var(--border-primary, #333)",
    borderRadius: 8,
    padding: "10px 14px",
    flex: "1 1 140px",
    minWidth: 140,
  } as React.CSSProperties,
  modeLabel: {
    fontSize: 13,
    fontWeight: 600,
    marginBottom: 4,
  } as React.CSSProperties,
  modeStat: {
    fontSize: 11,
    color: "var(--text-secondary, #999)",
  } as React.CSSProperties,
} as const;

const STATUS_COLORS: Record<string, string> = {
  running: "#4fc3f7",
  queued: "#ffb74d",
  paused: "#ce93d8",
  completed: "#81c784",
  failed: "#ef5350",
  cancelled: "#757575",
  idle: "#90a4ae",
  working: "#4fc3f7",
  stopped: "#757575",
  crashed: "#ef5350",
  starting: "#ffb74d",
  done: "#81c784",
  error: "#ef5350",
  active: "#4fc3f7",
  merged: "#81c784",
  conflict: "#ef5350",
};

function statusColor(s: string): string {
  return STATUS_COLORS[s?.toLowerCase()] ?? "#90a4ae";
}

function StatusBadge({ status }: { status: string }) {
  const c = statusColor(status);
  return <span style={S.badge(c)}>{status}</span>;
}

/* ── Component ────────────────────────────────────────────────────────────── */

export function AgentOSDashboard() {
  const [pool, setPool] = useState<PoolStats | null>(null);
  const [spawned, setSpawned] = useState<SpawnedAgent[]>([]);
  const [subAgents, setSubAgents] = useState<SubAgent[]>([]);
  const [hosted, setHosted] = useState<HostedAgent[]>([]);
  const [branches, setBranches] = useState<BranchAgent[]>([]);
  const [modeStats, setModeStats] = useState<ModeStats[]>([]);
  const [loading, setLoading] = useState(false);
  const [lastRefresh, setLastRefresh] = useState<Date | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const [poolRes, spawnRes, subRes, hostRes, branchRes, modesRes] = await Promise.allSettled([
        invoke<PoolStats>("spawn_agent_stats"),
        invoke<SpawnedAgent[]>("spawn_agent_list").then(r => Array.isArray(r) ? r : []),
        invoke<SubAgent[]>("list_sub_agents").then(r => Array.isArray(r) ? r : []),
        invoke<HostedAgent[]>("host_list_agents").then(r => Array.isArray(r) ? r : []),
        invoke<BranchAgent[]>("list_branch_agents").then(r => Array.isArray(r) ? r : []),
        invoke<ModeStats[]>("get_agent_mode_stats").then(r => Array.isArray(r) ? r : []),
      ]);
      if (poolRes.status === "fulfilled") setPool(poolRes.value);
      if (spawnRes.status === "fulfilled") setSpawned(spawnRes.value);
      if (subRes.status === "fulfilled") setSubAgents(subRes.value);
      if (hostRes.status === "fulfilled") setHosted(hostRes.value);
      if (branchRes.status === "fulfilled") setBranches(branchRes.value);
      if (modesRes.status === "fulfilled") setModeStats(modesRes.value);
      setLastRefresh(new Date());
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  // Auto-refresh every 10s
  useEffect(() => {
    const id = setInterval(refresh, 10_000);
    return () => clearInterval(id);
  }, [refresh]);

  const totalActive =
    (pool?.running ?? 0) +
    (pool?.queued ?? 0) +
    subAgents.filter(a => a.status === "working" || a.status === "running").length +
    hosted.filter(a => a.status === "Running" || a.status === "Starting").length +
    branches.filter(a => a.status === "active" || a.status === "running").length;

  return (
    <div className="panel-container">
      {/* Header */}
      <div className="panel-header">
        <div style={S.title}>
          <Cpu size={18} />
          <h3>Agent-OS Dashboard</h3>
        </div>
        <button className="panel-btn panel-btn-secondary" style={{ marginLeft: "auto" }} onClick={refresh} disabled={loading}>
          {loading ? <Loader2 size={12} className="spin" /> : <RefreshCw size={12} />}
          {lastRefresh ? `${lastRefresh.toLocaleTimeString()}` : "Refresh"}
        </button>
      </div>

      <div className="panel-body">
      {/* Pool Stats */}
      <div style={S.grid}>
        <div style={S.statCard}>
          <span style={S.statLabel}>Active</span>
          <span style={{ ...S.statValue, color: "#4fc3f7" }}>{totalActive}</span>
        </div>
        <div style={S.statCard}>
          <span style={S.statLabel}>Running</span>
          <span style={{ ...S.statValue, color: "#4fc3f7" }}>{pool?.running ?? 0}</span>
        </div>
        <div style={S.statCard}>
          <span style={S.statLabel}>Queued</span>
          <span style={{ ...S.statValue, color: "#ffb74d" }}>{pool?.queued ?? 0}</span>
        </div>
        <div style={S.statCard}>
          <span style={S.statLabel}>Completed</span>
          <span style={{ ...S.statValue, color: "#81c784" }}>{pool?.completed ?? 0}</span>
        </div>
        <div style={S.statCard}>
          <span style={S.statLabel}>Failed</span>
          <span style={{ ...S.statValue, color: "#ef5350" }}>{pool?.failed ?? 0}</span>
        </div>
        <div style={S.statCard}>
          <span style={S.statLabel}>Tokens Used</span>
          <span style={S.statValue}>{(pool?.total_tokens ?? 0).toLocaleString()}</span>
        </div>
      </div>

      {/* Spawned Agents */}
      <div style={S.section}>
        <div style={S.sectionTitle}>
          <Zap size={14} /> Spawned Agents ({spawned.length})
        </div>
        <table style={S.table}>
          <thead>
            <tr>
              <th style={S.th}>ID</th>
              <th style={S.th}>Task</th>
              <th style={S.th}>Status</th>
              <th style={S.th}>Priority</th>
              <th style={S.th}>Isolation</th>
              <th style={S.th}>Progress</th>
            </tr>
          </thead>
          <tbody>
            {spawned.length === 0 ? (
              <tr><td colSpan={6} style={S.emptyRow}>No spawned agents</td></tr>
            ) : spawned.map(a => (
              <tr key={a.id}>
                <td style={S.td}><code>{a.id.slice(0, 8)}</code></td>
                <td style={{ ...S.td, maxWidth: 200, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{a.task}</td>
                <td style={S.td}><StatusBadge status={a.status} /></td>
                <td style={S.td}>{a.priority}</td>
                <td style={S.td}>{a.isolation_mode}</td>
                <td style={S.td}>{a.progress ? `${a.progress.percent}%` : "-"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Sub-Agents */}
      <div style={S.section}>
        <div style={S.sectionTitle}>
          <Bot size={14} /> Sub-Agents ({subAgents.length})
        </div>
        <table style={S.table}>
          <thead>
            <tr>
              <th style={S.th}>ID</th>
              <th style={S.th}>Role</th>
              <th style={S.th}>Status</th>
              <th style={S.th}>Task</th>
              <th style={S.th}>Tokens</th>
            </tr>
          </thead>
          <tbody>
            {subAgents.length === 0 ? (
              <tr><td colSpan={5} style={S.emptyRow}>No sub-agents</td></tr>
            ) : subAgents.map(a => (
              <tr key={a.id}>
                <td style={S.td}><code>{a.id.slice(0, 8)}</code></td>
                <td style={S.td}>{a.role}</td>
                <td style={S.td}><StatusBadge status={a.status} /></td>
                <td style={{ ...S.td, maxWidth: 200, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{a.task ?? "-"}</td>
                <td style={S.td}>{a.tokens_used?.toLocaleString() ?? "-"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Hosted Agents */}
      <div style={S.section}>
        <div style={S.sectionTitle}>
          <Cpu size={14} /> Hosted Agents ({hosted.length})
        </div>
        <table style={S.table}>
          <thead>
            <tr>
              <th style={S.th}>ID</th>
              <th style={S.th}>Name</th>
              <th style={S.th}>Command</th>
              <th style={S.th}>Status</th>
            </tr>
          </thead>
          <tbody>
            {hosted.length === 0 ? (
              <tr><td colSpan={4} style={S.emptyRow}>No hosted agents</td></tr>
            ) : hosted.map(a => (
              <tr key={a.id}>
                <td style={S.td}><code>{a.id.slice(0, 8)}</code></td>
                <td style={S.td}>{a.name}</td>
                <td style={S.td}><code>{a.command}</code></td>
                <td style={S.td}><StatusBadge status={a.status} /></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Branch Agents */}
      <div style={S.section}>
        <div style={S.sectionTitle}>
          <GitBranch size={14} /> Branch Agents ({branches.length})
        </div>
        <table style={S.table}>
          <thead>
            <tr>
              <th style={S.th}>ID</th>
              <th style={S.th}>Branch</th>
              <th style={S.th}>Task</th>
              <th style={S.th}>Status</th>
            </tr>
          </thead>
          <tbody>
            {branches.length === 0 ? (
              <tr><td colSpan={4} style={S.emptyRow}>No branch agents</td></tr>
            ) : branches.map(a => (
              <tr key={a.id}>
                <td style={S.td}><code>{a.id.slice(0, 8)}</code></td>
                <td style={S.td}><code>{a.branch}</code></td>
                <td style={{ ...S.td, maxWidth: 200, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{a.task}</td>
                <td style={S.td}><StatusBadge status={a.status} /></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Mode Stats */}
      {modeStats.length > 0 && (
        <div style={S.section}>
          <div style={S.sectionTitle}>
            <BarChart3 size={14} /> Agent Modes
          </div>
          <div style={S.modeBar}>
            {modeStats.map(m => (
              <div key={m.modeId} style={S.modeCard}>
                <div style={S.modeLabel}>{m.modeId}</div>
                <div style={S.modeStat}>{m.invocations} runs</div>
                <div style={S.modeStat}>~{(m.avgTokens ?? 0).toLocaleString()} avg tokens</div>
                <div style={S.modeStat}>{m.lastUsed ? `Last: ${m.lastUsed.slice(0, 10)}` : "Never used"}</div>
              </div>
            ))}
          </div>
        </div>
      )}
      </div>
    </div>
  );
}
