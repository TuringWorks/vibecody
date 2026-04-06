/**
 * CompanyGoalsPanel — Hierarchical goal tree with progress bars.
 *
 * Shows company → team → agent goal hierarchy. Supports creating,
 * updating progress, and changing goal status.
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface CompanyGoalsPanelProps {
  workspacePath?: string | null;
}

export function CompanyGoalsPanel({ workspacePath: _wp }: CompanyGoalsPanelProps) {
  const [output, setOutput] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [newTitle, setNewTitle] = useState("");
  const [cmdResult, setCmdResult] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    try {
      const out = await invoke<string>("company_cmd", { args: "goal list" });
      setOutput(out);
    } catch (e) {
      setOutput(`Error: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  const createGoal = async () => {
    if (!newTitle.trim()) return;
    try {
      const out = await invoke<string>("company_cmd", { args: `goal create ${newTitle.trim()}` });
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
        <span style={{ fontWeight: 600, fontSize: 14 }}>Goals</span>
        <button onClick={load} style={{ fontSize: 11, padding: "2px 8px", cursor: "pointer" }}>
          Refresh
        </button>
      </div>

      {/* Create goal */}
      <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
        <input
          value={newTitle}
          onChange={(e) => setNewTitle(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && createGoal()}
          placeholder="New goal title…"
          style={{
            flex: 1, fontSize: 12, padding: "4px 8px",
            background: "var(--input-bg, rgba(0,0,0,0.3))",
            border: "1px solid var(--border)", borderRadius: 4,
            color: "var(--text-primary)",
          }}
        />
        <button onClick={createGoal} style={{ fontSize: 11, padding: "4px 12px", cursor: "pointer" }}>
          + Goal
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
            {output || "No goals yet.\nUse /company goal create <title> in REPL or the form above."}
          </pre>
        )}
      </div>

      <div style={{ marginTop: 12, fontSize: 11, color: "var(--text-secondary)" }}>
        Goal statuses: planned → active → achieved | cancelled
      </div>
    </div>
  );
}
