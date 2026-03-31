import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface WorktreeAgent {
  id: string;
  branch: string;
  status: "running" | "merging" | "done" | "failed";
  progress: number;
  filesChanged: number;
  startedAt: string;
  duration?: string;
}

interface QueueItem {
  id: string;
  branch: string;
  hasConflicts: boolean;
  position: number;
}

const panelStyle: React.CSSProperties = {
  padding: 16,
  height: "100%",
  overflow: "auto",
  color: "var(--text-primary)",
  background: "var(--bg-primary)",
};

const headingStyle: React.CSSProperties = {
  fontSize: 18,
  fontWeight: 600,
  marginBottom: 12,
  color: "var(--text-primary)",
};

const cardStyle: React.CSSProperties = {
  background: "var(--bg-secondary)",
  borderRadius: 8,
  padding: 12,
  marginBottom: 8,
  border: "1px solid var(--border-color)",
};

const btnStyle: React.CSSProperties = {
  padding: "6px 14px",
  borderRadius: 6,
  border: "1px solid var(--border-color)",
  background: "var(--accent-color)",
  color: "var(--btn-primary-fg, #fff)",
  cursor: "pointer",
  fontSize: 13,
  marginRight: 8,
};

const tabStyle = (active: boolean): React.CSSProperties => ({
  padding: "8px 16px",
  cursor: "pointer",
  borderBottom: active ? "2px solid var(--accent-color)" : "2px solid transparent",
  color: active ? "var(--accent-color)" : "var(--text-secondary)",
  background: "transparent",
  border: "none",
  fontSize: 13,
  fontWeight: active ? 600 : 400,
});

const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block",
  padding: "2px 8px",
  borderRadius: 10,
  fontSize: 11,
  fontWeight: 600,
  background: color,
  color: "var(--btn-primary-fg, #fff)",
});

const statusColor: Record<string, string> = { running: "var(--accent-color)", merging: "var(--warning-color)", done: "var(--success-color)", failed: "var(--error-color)" };

export function WorktreePoolPanel() {
  const [tab, setTab] = useState("active");
  const [agents, setAgents] = useState<WorktreeAgent[]>([]);
  const [queue, setQueue] = useState<QueueItem[]>([]);
  const [maxWorktrees, setMaxWorktrees] = useState(4);
  const [baseBranch, setBaseBranch] = useState("main");
  const [autoCleanup, setAutoCleanup] = useState(true);
  const [loading, setLoading] = useState(true);
  const [actionLoading, setActionLoading] = useState<string | null>(null);
  const [spawnTask, setSpawnTask] = useState("");

  useEffect(() => {
    async function loadWorktrees() {
      setLoading(true);
      try {
        const res = await invoke<{ agents: WorktreeAgent[]; active_count: number }>("worktree_list");
        const list = Array.isArray(res) ? res : (res.agents ?? []);
        setAgents(list);
        // Derive queue from agents that are in merging state
        const mergeQueue: QueueItem[] = list
          .filter((a) => a.status === "merging")
          .map((a, i) => ({ id: a.id, branch: a.branch, hasConflicts: false, position: i + 1 }));
        setQueue(mergeQueue);
      } catch (e) {
        console.error("Failed to load worktrees:", e);
      }
      setLoading(false);
    }
    loadWorktrees();
  }, []);

  const handleSpawn = async () => {
    if (!spawnTask.trim()) return;
    setActionLoading("spawn");
    try {
      await invoke("worktree_spawn", { task: spawnTask });
      setSpawnTask("");
      // Refresh list
      const r = await invoke<{ agents: WorktreeAgent[]; active_count: number }>("worktree_list");
      setAgents(Array.isArray(r) ? r : (r.agents ?? []));
    } catch (e) {
      console.error("Failed to spawn worktree:", e);
    }
    setActionLoading(null);
  };

  const handleMerge = async (agentId: string) => {
    setActionLoading(agentId);
    try {
      await invoke("worktree_merge", { agentId });
      const r2 = await invoke<{ agents: WorktreeAgent[]; active_count: number }>("worktree_list");
      setAgents(Array.isArray(r2) ? r2 : (r2.agents ?? []));
    } catch (e) {
      console.error("Failed to merge worktree:", e);
    }
    setActionLoading(null);
  };

  const handleCleanup = async () => {
    setActionLoading("cleanup");
    try {
      await invoke("worktree_cleanup");
      const r3 = await invoke<{ agents: WorktreeAgent[]; active_count: number }>("worktree_list");
      setAgents(Array.isArray(r3) ? r3 : (r3.agents ?? []));
    } catch (e) {
      console.error("Failed to cleanup worktrees:", e);
    }
    setActionLoading(null);
  };

  const inputStyle: React.CSSProperties = { width: "100%", padding: 8, borderRadius: 6, border: "1px solid var(--border-color)", background: "var(--bg-primary)", color: "var(--text-primary)", fontSize: 13 };

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Parallel Worktree Agents</h2>
      <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
        <input style={{ ...inputStyle, flex: 1 }} placeholder="Task description for new worktree..." value={spawnTask} onChange={(e) => setSpawnTask(e.target.value)} />
        <button style={btnStyle} onClick={handleSpawn} disabled={actionLoading === "spawn"}>
          {actionLoading === "spawn" ? "Spawning..." : "Spawn"}
        </button>
        <button style={{ ...btnStyle, background: "var(--bg-secondary)", color: "var(--text-primary)" }} onClick={handleCleanup} disabled={actionLoading === "cleanup"}>
          {actionLoading === "cleanup" ? "Cleaning..." : "Cleanup"}
        </button>
      </div>
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--border-color)", marginBottom: 16 }}>
        <button style={tabStyle(tab === "active")} onClick={() => setTab("active")}>Active</button>
        <button style={tabStyle(tab === "queue")} onClick={() => setTab("queue")}>Queue</button>
        <button style={tabStyle(tab === "history")} onClick={() => setTab("history")}>History</button>
        <button style={tabStyle(tab === "config")} onClick={() => setTab("config")}>Config</button>
      </div>

      {tab === "active" && (
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
          {loading && <div style={{ color: "var(--text-secondary)", fontSize: 13, gridColumn: "1 / -1" }}>Loading worktrees...</div>}
          {!loading && agents.filter((a) => a.status === "running" || a.status === "merging").length === 0 && (
            <div style={{ color: "var(--text-secondary)", fontSize: 13, gridColumn: "1 / -1" }}>No active worktrees. Spawn a task to get started.</div>
          )}
          {agents.filter((a) => a.status === "running" || a.status === "merging").map((a) => (
            <div key={a.id} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
                <strong style={{ fontSize: 13 }}>{a.branch}</strong>
                <span style={badgeStyle(statusColor[a.status])}>{a.status}</span>
              </div>
              <div style={{ background: "var(--bg-primary)", borderRadius: 4, height: 8, marginBottom: 6 }}>
                <div style={{ background: statusColor[a.status], borderRadius: 4, height: 8, width: `${a.progress}%`, transition: "width 0.3s" }} />
              </div>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>{a.filesChanged} files changed | Started {a.startedAt}</div>
                {a.status === "running" && (
                  <button style={{ ...btnStyle, fontSize: 11, padding: "4px 10px" }} onClick={() => handleMerge(a.id)} disabled={actionLoading === a.id}>
                    {actionLoading === a.id ? "..." : "Merge"}
                  </button>
                )}
              </div>
            </div>
          ))}
        </div>
      )}

      {tab === "queue" && (
        <div>
          {loading && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>Loading queue...</div>}
          {!loading && queue.length === 0 && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>Merge queue is empty</div>}
          {queue.map((q) => (
            <div key={q.id} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <span><strong>#{q.position}</strong> {q.branch}</span>
                {q.hasConflicts && <span style={badgeStyle("var(--error-color)")}>conflicts</span>}
                {!q.hasConflicts && <span style={badgeStyle("var(--success-color)")}>clean</span>}
              </div>
            </div>
          ))}
        </div>
      )}

      {tab === "history" && (
        <div>
          {loading && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>Loading history...</div>}
          {!loading && agents.filter((a) => a.status === "done" || a.status === "failed").length === 0 && (
            <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>No completed worktrees yet.</div>
          )}
          {agents.filter((a) => a.status === "done" || a.status === "failed").map((a) => (
            <div key={a.id} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <strong>{a.branch}</strong>
                <span style={badgeStyle(statusColor[a.status])}>{a.status}</span>
              </div>
              <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 4 }}>{a.filesChanged} files | {a.duration || "N/A"}</div>
            </div>
          ))}
        </div>
      )}

      {tab === "config" && (
        <div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, marginBottom: 8 }}>Max Worktrees: {maxWorktrees}</div>
            <input type="range" min={1} max={8} value={maxWorktrees} onChange={(e) => setMaxWorktrees(Number(e.target.value))} style={{ width: "100%" }} />
          </div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, marginBottom: 8 }}>Base Branch</div>
            <input style={inputStyle} value={baseBranch} onChange={(e) => setBaseBranch(e.target.value)} />
          </div>
          <div style={cardStyle}>
            <label style={{ display: "flex", alignItems: "center", gap: 8, cursor: "pointer" }}>
              <input type="checkbox" checked={autoCleanup} onChange={(e) => setAutoCleanup(e.target.checked)} />
              <span style={{ fontWeight: 600 }}>Auto-cleanup completed worktrees</span>
            </label>
          </div>
        </div>
      )}
    </div>
  );
}
