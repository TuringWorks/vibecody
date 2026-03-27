import { useState } from "react";

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
  color: "#fff",
});

const statusColor: Record<string, string> = { running: "#3b82f6", merging: "#f59e0b", done: "#22c55e", failed: "#ef4444" };

export function WorktreePoolPanel() {
  const [tab, setTab] = useState("active");
  const [agents] = useState<WorktreeAgent[]>([
    { id: "w1", branch: "feat/auth-refactor", status: "running", progress: 65, filesChanged: 8, startedAt: "10:15" },
    { id: "w2", branch: "fix/api-timeout", status: "merging", progress: 100, filesChanged: 3, startedAt: "09:45" },
    { id: "w3", branch: "feat/dashboard", status: "done", progress: 100, filesChanged: 12, startedAt: "08:30", duration: "1h 42m" },
  ]);
  const [queue] = useState<QueueItem[]>([
    { id: "q1", branch: "feat/auth-refactor", hasConflicts: false, position: 1 },
    { id: "q2", branch: "fix/api-timeout", hasConflicts: true, position: 2 },
  ]);
  const [maxWorktrees, setMaxWorktrees] = useState(4);
  const [baseBranch, setBaseBranch] = useState("main");
  const [autoCleanup, setAutoCleanup] = useState(true);

  const inputStyle: React.CSSProperties = { width: "100%", padding: 8, borderRadius: 6, border: "1px solid var(--border-color)", background: "var(--bg-primary)", color: "var(--text-primary)", fontSize: 13 };

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Parallel Worktree Agents</h2>
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--border-color)", marginBottom: 16 }}>
        <button style={tabStyle(tab === "active")} onClick={() => setTab("active")}>Active</button>
        <button style={tabStyle(tab === "queue")} onClick={() => setTab("queue")}>Queue</button>
        <button style={tabStyle(tab === "history")} onClick={() => setTab("history")}>History</button>
        <button style={tabStyle(tab === "config")} onClick={() => setTab("config")}>Config</button>
      </div>

      {tab === "active" && (
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
          {agents.filter((a) => a.status === "running" || a.status === "merging").map((a) => (
            <div key={a.id} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
                <strong style={{ fontSize: 13 }}>{a.branch}</strong>
                <span style={badgeStyle(statusColor[a.status])}>{a.status}</span>
              </div>
              <div style={{ background: "var(--bg-primary)", borderRadius: 4, height: 8, marginBottom: 6 }}>
                <div style={{ background: statusColor[a.status], borderRadius: 4, height: 8, width: `${a.progress}%`, transition: "width 0.3s" }} />
              </div>
              <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>{a.filesChanged} files changed | Started {a.startedAt}</div>
            </div>
          ))}
        </div>
      )}

      {tab === "queue" && (
        <div>
          {queue.map((q) => (
            <div key={q.id} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <span><strong>#{q.position}</strong> {q.branch}</span>
                {q.hasConflicts && <span style={badgeStyle("#ef4444")}>conflicts</span>}
                {!q.hasConflicts && <span style={badgeStyle("#22c55e")}>clean</span>}
              </div>
            </div>
          ))}
          {queue.length === 0 && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>Merge queue is empty</div>}
        </div>
      )}

      {tab === "history" && (
        <div>
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
