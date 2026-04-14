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
    <div className="panel-container">
      <div className="panel-header">
        <h3>Agent Goals</h3>
        <div style={{ display: "flex", gap: 6, marginLeft: "auto" }}>
          {(["list", "tree", "create"] as const).map((v) => (
            <button key={v} onClick={() => setView(v)} className={`panel-btn ${view === v ? "panel-btn-primary" : "panel-btn-secondary"}`} style={{ padding: "2px 8px" }}>
              {v === "create" ? "+ New" : v.charAt(0).toUpperCase() + v.slice(1)}
            </button>
          ))}
          <button onClick={load} className="panel-btn panel-btn-secondary">Refresh</button>
        </div>
      </div>
      <div className="panel-body">

      {cmdResult && (
        <div className="panel-card" style={{ marginBottom: 12, fontSize: "var(--font-size-base)" }}>
          {cmdResult}
          <button onClick={() => setCmdResult(null)} style={{ marginLeft: 8, cursor: "pointer", background: "none", border: "none", color: "var(--text-secondary)", display: "inline-flex" }}><X size={12} /></button>
        </div>
      )}

      {/* Create form */}
      {view === "create" && (
        <div className="panel-card" style={{ marginBottom: 12 }}>
          <div style={{ fontWeight: 600, marginBottom: 8 }}>New Goal</div>
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            <input value={newTitle} onChange={(e) => setNewTitle(e.target.value)} onKeyDown={(e) => e.key === "Enter" && createGoal()} placeholder="Goal title *" autoFocus className="panel-input panel-input-full" />
            <input value={newDesc} onChange={(e) => setNewDesc(e.target.value)} placeholder="Description (optional)" className="panel-input panel-input-full" />
            <button onClick={createGoal} disabled={creating || !newTitle.trim()} className="panel-btn panel-btn-primary" style={{ opacity: creating ? 0.6 : 1, alignSelf: "flex-start" }}>
              {creating ? "Creating…" : "Create Goal"}
            </button>
          </div>
        </div>
      )}

      {/* List view */}
      {view === "list" && (
        isEmpty && !loading ? (
          <div className="panel-empty" style={{ padding: 24 }}>
            <div style={{ marginBottom: 8, display: "flex", justifyContent: "center", color: "var(--accent, #4a9eff)" }}><Target size={32} strokeWidth={1.5} /></div>
            <div style={{ fontWeight: 600, marginBottom: 4 }}>No goals yet</div>
            <div style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-base)", marginBottom: 16 }}>
              Set company goals to track progress
            </div>
            <button onClick={() => setView("create")} className="panel-btn panel-btn-primary" style={{ fontSize: "var(--font-size-base)" }}>+ Create Goal</button>
          </div>
        ) : (
          <div className="panel-card" style={{ minHeight: 120 }}>
            {loading ? (
              <span className="panel-loading">Loading…</span>
            ) : (
              <pre style={{ margin: 0, fontSize: "var(--font-size-base)", whiteSpace: "pre-wrap", lineHeight: 1.7, fontFamily: "inherit" }}>{output}</pre>
            )}
          </div>
        )
      )}

      {/* Tree view */}
      {view === "tree" && (
        <div className="panel-card" style={{ minHeight: 120 }}>
          {loading ? (
            <span className="panel-loading">Loading…</span>
          ) : (
            <pre style={{ margin: 0, fontSize: "var(--font-size-base)", whiteSpace: "pre-wrap", lineHeight: 1.7, fontFamily: "inherit" }}>
              {treeOutput || "No goals yet."}
            </pre>
          )}
        </div>
      )}

      <div style={{ marginTop: 10, fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
        Workflow: planned → active → achieved | cancelled
      </div>
      </div>
    </div>
  );
}
