/**
 * CompanyGoalsPanel — Hierarchical goal tree with progress tracking.
 *
 * Create goals, view list or tree view. Shows status and progress %.
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Target, X } from "lucide-react";

interface CompanyGoalsPanelProps {
  workspacePath?: string | null;
}

const btnStyle: React.CSSProperties = {
  fontSize: 11, padding: "3px 10px", cursor: "pointer", borderRadius: 4,
  background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", color: "var(--text-primary)",
};

const inputStyle: React.CSSProperties = {
  fontSize: 12, padding: "4px 8px", background: "var(--bg-primary)",
  border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)",
};

export function CompanyGoalsPanel({ workspacePath: _wp }: CompanyGoalsPanelProps) {
  const [output, setOutput] = useState<string>("");
  const [treeOutput, setTreeOutput] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [view, setView] = useState<"list" | "tree" | "create">("list");
  const [newTitle, setNewTitle] = useState("");
  const [newDesc, setNewDesc] = useState("");
  const [cmdResult, setCmdResult] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);

  const load = async () => {
    setLoading(true);
    try {
      const [list, tree] = await Promise.all([
        invoke<string>("company_cmd", { args: "goal list" }),
        invoke<string>("company_cmd", { args: "goal tree" }),
      ]);
      setOutput(list);
      setTreeOutput(tree);
    } catch (e) {
      setOutput(`Error: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  const createGoal = async () => {
    if (!newTitle.trim()) return;
    setCreating(true);
    try {
      const out = await invoke<string>("company_cmd", { args: `goal create ${newTitle.trim()}` });
      setCmdResult(out);
      setNewTitle("");
      setNewDesc("");
      setView("list");
      load();
    } catch (e) {
      setCmdResult(`Error: ${e}`);
    } finally {
      setCreating(false);
    }
  };

  const isEmpty = !output || output.includes("No goals");

  return (
    <div style={{ padding: 16, fontSize: 13, height: "100%", overflowY: "auto" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
        <span style={{ fontWeight: 600, fontSize: 14 }}>Agent Goals</span>
        <div style={{ display: "flex", gap: 6 }}>
          {(["list", "tree", "create"] as const).map((v) => (
            <button key={v} onClick={() => setView(v)} style={{
              ...btnStyle, padding: "2px 8px",
              background: view === v ? "var(--accent, #4a9eff)" : "var(--bg-tertiary)",
              color: view === v ? "#fff" : "var(--text-primary)",
              border: `1px solid ${view === v ? "var(--accent, #4a9eff)" : "var(--border-color)"}`,
            }}>
              {v === "create" ? "+ New" : v.charAt(0).toUpperCase() + v.slice(1)}
            </button>
          ))}
          <button onClick={load} style={btnStyle}>Refresh</button>
        </div>
      </div>

      {cmdResult && (
        <div style={{ background: "var(--panel-bg, rgba(0,0,0,0.2))", border: "1px solid var(--border-color)", borderRadius: 4, padding: 8, marginBottom: 12, fontSize: 12 }}>
          {cmdResult}
          <button onClick={() => setCmdResult(null)} style={{ marginLeft: 8, cursor: "pointer", background: "none", border: "none", color: "var(--text-secondary)", display: "inline-flex" }}><X size={12} /></button>
        </div>
      )}

      {/* Create form */}
      {view === "create" && (
        <div style={{ background: "var(--panel-bg, rgba(0,0,0,0.2))", border: "1px solid var(--border-color)", borderRadius: 6, padding: 12, marginBottom: 12 }}>
          <div style={{ fontWeight: 600, marginBottom: 8 }}>New Goal</div>
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            <input value={newTitle} onChange={(e) => setNewTitle(e.target.value)} onKeyDown={(e) => e.key === "Enter" && createGoal()} placeholder="Goal title *" autoFocus style={{ ...inputStyle, width: "100%", boxSizing: "border-box" }} />
            <input value={newDesc} onChange={(e) => setNewDesc(e.target.value)} placeholder="Description (optional)" style={{ ...inputStyle, width: "100%", boxSizing: "border-box" }} />
            <button onClick={createGoal} disabled={creating || !newTitle.trim()} style={{ ...btnStyle, padding: "5px 16px", opacity: creating ? 0.6 : 1, alignSelf: "flex-start" }}>
              {creating ? "Creating…" : "Create Goal"}
            </button>
          </div>
        </div>
      )}

      {/* List view */}
      {view === "list" && (
        isEmpty && !loading ? (
          <div style={{ background: "var(--panel-bg, rgba(0,0,0,0.2))", border: "1px solid var(--border-color)", borderRadius: 6, padding: 24, textAlign: "center" }}>
            <div style={{ marginBottom: 8, display: "flex", justifyContent: "center", color: "var(--accent, #4a9eff)" }}><Target size={32} strokeWidth={1.5} /></div>
            <div style={{ fontWeight: 600, marginBottom: 4 }}>No goals yet</div>
            <div style={{ color: "var(--text-secondary)", fontSize: 12, marginBottom: 16 }}>
              Set company goals to track progress
            </div>
            <button onClick={() => setView("create")} style={{ ...btnStyle, padding: "6px 20px", fontSize: 12 }}>+ Create Goal</button>
          </div>
        ) : (
          <div style={{ background: "var(--panel-bg, rgba(0,0,0,0.2))", border: "1px solid var(--border-color)", borderRadius: 6, padding: 12, minHeight: 120 }}>
            {loading ? (
              <span style={{ color: "var(--text-secondary)" }}>Loading…</span>
            ) : (
              <pre style={{ margin: 0, fontSize: 12, whiteSpace: "pre-wrap", lineHeight: 1.7, fontFamily: "inherit" }}>{output}</pre>
            )}
          </div>
        )
      )}

      {/* Tree view */}
      {view === "tree" && (
        <div style={{ background: "var(--panel-bg, rgba(0,0,0,0.2))", border: "1px solid var(--border-color)", borderRadius: 6, padding: 12, minHeight: 120 }}>
          {loading ? (
            <span style={{ color: "var(--text-secondary)" }}>Loading…</span>
          ) : (
            <pre style={{ margin: 0, fontSize: 12, whiteSpace: "pre-wrap", lineHeight: 1.7, fontFamily: "inherit" }}>
              {treeOutput || "No goals yet."}
            </pre>
          )}
        </div>
      )}

      <div style={{ marginTop: 10, fontSize: 11, color: "var(--text-secondary)" }}>
        Workflow: planned → active → achieved | cancelled
      </div>
    </div>
  );
}
