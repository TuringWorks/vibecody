import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface DashboardData {
  project_name: string;
  languages: string[];
  total_files: number;
  total_lines: number;
  git_branch: string;
  git_uncommitted: number;
  recent_commits: number;
  test_framework: string;
  has_ci: boolean;
  open_todos: number;
  agent_sessions: number;
}

const DashboardPanel: React.FC = () => {
  const [data, setData] = useState<DashboardData | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<DashboardData>("get_project_dashboard");
      setData(result);
    } catch (e: unknown) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    refresh();
  }, []);

  return (
    <div style={{ padding: 16, fontFamily: "var(--font-mono, monospace)", color: "var(--fg, #e0e0e0)" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 16 }}>
        <h3 style={{ margin: 0 }}>Project Dashboard</h3>
        <button
          onClick={refresh}
          disabled={loading}
          style={{
            padding: "4px 12px",
            background: "var(--accent, #4fc3f7)",
            color: "#000",
            border: "none",
            borderRadius: 4,
            cursor: loading ? "wait" : "pointer",
            fontSize: 12,
          }}
        >
          {loading ? "Scanning..." : "Refresh"}
        </button>
      </div>

      {error && (
        <div style={{ padding: 8, background: "#3a1c1c", borderRadius: 4, marginBottom: 12, fontSize: 12 }}>
          {error}
        </div>
      )}

      {data && (
        <>
          {/* Summary Cards */}
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(140px, 1fr))", gap: 8, marginBottom: 16 }}>
            <Card label="Project" value={data.project_name} />
            <Card label="Branch" value={data.git_branch || "N/A"} />
            <Card label="Files" value={String(data.total_files)} />
            <Card label="Lines" value={formatNum(data.total_lines)} />
            <Card label="Uncommitted" value={String(data.git_uncommitted)} color={data.git_uncommitted > 0 ? "#ffb74d" : "#81c784"} />
            <Card label="Recent Commits" value={String(data.recent_commits)} />
            <Card label="TODOs" value={String(data.open_todos)} color={data.open_todos > 5 ? "#ef5350" : "#81c784"} />
            <Card label="Agent Sessions" value={String(data.agent_sessions)} />
          </div>

          {/* Languages */}
          <div style={{ marginBottom: 12 }}>
            <div style={{ fontSize: 11, color: "#888", marginBottom: 4 }}>Languages</div>
            <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
              {data.languages.map((lang) => (
                <span
                  key={lang}
                  style={{
                    padding: "2px 8px",
                    background: "#333",
                    borderRadius: 12,
                    fontSize: 11,
                  }}
                >
                  {lang}
                </span>
              ))}
            </div>
          </div>

          {/* Status Badges */}
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
            <Badge label="Tests" value={data.test_framework || "None"} ok={!!data.test_framework} />
            <Badge label="CI" value={data.has_ci ? "Configured" : "None"} ok={data.has_ci} />
          </div>
        </>
      )}

      {!data && !loading && !error && (
        <div style={{ color: "#888", textAlign: "center", padding: 32 }}>
          Click Refresh to scan the project.
        </div>
      )}
    </div>
  );
};

const Card: React.FC<{ label: string; value: string; color?: string }> = ({ label, value, color }) => (
  <div style={{
    padding: "8px 12px",
    background: "var(--bg-tertiary)",
    borderRadius: 6,
    border: "1px solid #333",
  }}>
    <div style={{ fontSize: 10, color: "#888", marginBottom: 2 }}>{label}</div>
    <div style={{ fontSize: 16, fontWeight: 600, color: color || "#e0e0e0" }}>{value}</div>
  </div>
);

const Badge: React.FC<{ label: string; value: string; ok: boolean }> = ({ label, value, ok }) => (
  <span style={{
    padding: "3px 10px",
    borderRadius: 12,
    fontSize: 11,
    background: ok ? "#1b5e20" : "#4a1515",
    color: ok ? "#81c784" : "#ef5350",
  }}>
    {ok ? "\u2713" : "\u2717"} {label}: {value}
  </span>
);

function formatNum(n: number): string {
  if (n >= 1000000) return `${(n / 1000000).toFixed(1)}M`;
  if (n >= 1000) return `${(n / 1000).toFixed(1)}K`;
  return String(n);
}

export default DashboardPanel;
