/**
 * CompanyTaskBoardPanel — Kanban task board with status columns.
 *
 * Shows tasks grouped by status: Backlog, Todo, In Progress, In Review,
 * Done, Blocked. Supports creating tasks and transitioning status.
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

const STATUSES = ["backlog", "todo", "in_progress", "in_review", "done", "blocked"] as const;

interface CompanyTaskBoardPanelProps {
  workspacePath?: string | null;
}

export function CompanyTaskBoardPanel({ workspacePath: _wp }: CompanyTaskBoardPanelProps) {
  const [output, setOutput] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [newTitle, setNewTitle] = useState("");
  const [cmdResult, setCmdResult] = useState<string | null>(null);
  const [filterStatus, setFilterStatus] = useState<string>("");

  const load = async () => {
    setLoading(true);
    try {
      const args = filterStatus ? `task list --status ${filterStatus}` : "task list";
      const out = await invoke<string>("company_cmd", { args });
      setOutput(out);
    } catch (e) {
      setOutput(`Error: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, [filterStatus]);

  const createTask = async () => {
    if (!newTitle.trim()) return;
    try {
      const out = await invoke<string>("company_cmd", { args: `task create ${newTitle.trim()}` });
      setCmdResult(out);
      setNewTitle("");
      load();
    } catch (e) {
      setCmdResult(`Error: ${e}`);
    }
  };

  return (
    <div style={{ padding: 16, fontSize: 13, height: "100%", overflowY: "auto" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
        <span style={{ fontWeight: 600, fontSize: 14 }}>Task Board</span>
        <button onClick={load} style={{ fontSize: 11, padding: "2px 8px", cursor: "pointer" }}>
          Refresh
        </button>
      </div>

      {/* Filter by status */}
      <div style={{ display: "flex", gap: 6, marginBottom: 12, flexWrap: "wrap" }}>
        <button
          onClick={() => setFilterStatus("")}
          style={{
            fontSize: 11, padding: "2px 8px", cursor: "pointer",
            background: filterStatus === "" ? "var(--accent, #4a9eff)" : undefined,
            color: filterStatus === "" ? "#fff" : undefined,
            borderRadius: 12,
          }}
        >
          All
        </button>
        {STATUSES.map((s) => (
          <button
            key={s}
            onClick={() => setFilterStatus(s)}
            style={{
              fontSize: 11, padding: "2px 8px", cursor: "pointer", borderRadius: 12,
              background: filterStatus === s ? "var(--accent, #4a9eff)" : undefined,
              color: filterStatus === s ? "#fff" : undefined,
            }}
          >
            {s.replace("_", " ")}
          </button>
        ))}
      </div>

      {/* Create task */}
      <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
        <input
          value={newTitle}
          onChange={(e) => setNewTitle(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && createTask()}
          placeholder="New task title…"
          style={{
            flex: 1, fontSize: 12, padding: "4px 8px",
            background: "var(--input-bg, rgba(0,0,0,0.3))",
            border: "1px solid var(--border)", borderRadius: 4,
            color: "var(--text-primary)",
          }}
        />
        <button onClick={createTask} style={{ fontSize: 11, padding: "4px 12px", cursor: "pointer" }}>
          + Task
        </button>
      </div>

      {cmdResult && (
        <div style={{
          background: "var(--panel-bg, rgba(0,0,0,0.2))", border: "1px solid var(--border)",
          borderRadius: 4, padding: 8, marginBottom: 12, fontSize: 12,
        }}>
          {cmdResult}
        </div>
      )}

      <div style={{
        background: "var(--panel-bg, rgba(0,0,0,0.2))", border: "1px solid var(--border)",
        borderRadius: 6, padding: 12, minHeight: 200,
      }}>
        {loading ? (
          <span style={{ color: "var(--text-secondary)" }}>Loading…</span>
        ) : (
          <pre style={{ margin: 0, fontSize: 12, whiteSpace: "pre-wrap", lineHeight: 1.6 }}>
            {output || "No tasks yet.\nWorkflow: backlog → todo → in_progress → in_review → done"}
          </pre>
        )}
      </div>
    </div>
  );
}
