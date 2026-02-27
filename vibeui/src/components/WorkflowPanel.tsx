import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ChecklistItem {
  id: number;
  description: string;
  done: boolean;
}

interface WorkflowStage {
  stage: string;
  label: string;
  status: string;
  checklist: ChecklistItem[];
  body: string;
}

interface Workflow {
  name: string;
  description: string;
  current_stage: number;
  stages: WorkflowStage[];
  created_at: string;
  overall_progress: number;
}

interface WorkflowPanelProps {
  workspacePath: string | null;
  provider?: string;
}

const STATUS_COLORS: Record<string, string> = {
  "not-started": "#666",
  "in-progress": "#ff9800",
  complete: "#4caf50",
  skipped: "#9e9e9e",
};

const STAGE_ICONS = ["📝", "🏛️", "🎯", "🔧", "💻", "✅", "🔗", "🎉"];

export function WorkflowPanel({ workspacePath, provider = "ollama" }: WorkflowPanelProps) {
  const [workflows, setWorkflows] = useState<Workflow[]>([]);
  const [selected, setSelected] = useState<Workflow | null>(null);
  const [activeStage, setActiveStage] = useState<number>(0);
  const [view, setView] = useState<"list" | "detail">("list");
  const [loading, setLoading] = useState(false);
  const [generating, setGenerating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // New workflow form
  const [showNewForm, setShowNewForm] = useState(false);
  const [newName, setNewName] = useState("");
  const [newDesc, setNewDesc] = useState("");

  const loadWorkflows = useCallback(async () => {
    if (!workspacePath) return;
    try {
      setLoading(true);
      const list = await invoke<Workflow[]>("list_workflows", { workspacePath });
      setWorkflows(list);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [workspacePath]);

  useEffect(() => {
    loadWorkflows();
  }, [loadWorkflows]);

  const openWorkflow = async (name: string) => {
    if (!workspacePath) return;
    try {
      const w = await invoke<Workflow>("get_workflow", { workspacePath, name });
      setSelected(w);
      setActiveStage(w.current_stage);
      setView("detail");
    } catch (e) {
      setError(String(e));
    }
  };

  const createWorkflow = async () => {
    if (!workspacePath || !newName.trim() || !newDesc.trim()) return;
    try {
      setLoading(true);
      const w = await invoke<Workflow>("create_workflow", {
        workspacePath,
        name: newName.trim().replace(/\s+/g, "_").toLowerCase(),
        description: newDesc.trim(),
      });
      setWorkflows((prev) => [...prev, w]);
      setSelected(w);
      setActiveStage(0);
      setView("detail");
      setShowNewForm(false);
      setNewName("");
      setNewDesc("");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const toggleItem = async (stageIndex: number, itemId: number) => {
    if (!workspacePath || !selected) return;
    const item = selected.stages[stageIndex]?.checklist.find((c) => c.id === itemId);
    if (!item) return;
    try {
      const updated = await invoke<Workflow>("update_workflow_checklist_item", {
        workspacePath,
        name: selected.name,
        stageIndex,
        itemId,
        done: !item.done,
      });
      setSelected(updated);
      setWorkflows((prev) => prev.map((w) => (w.name === updated.name ? updated : w)));
    } catch (e) {
      setError(String(e));
    }
  };

  const advanceStage = async () => {
    if (!workspacePath || !selected) return;
    try {
      const updated = await invoke<Workflow>("advance_workflow_stage", {
        workspacePath,
        name: selected.name,
      });
      setSelected(updated);
      setActiveStage(updated.current_stage);
      setWorkflows((prev) => prev.map((w) => (w.name === updated.name ? updated : w)));
    } catch (e) {
      setError(String(e));
    }
  };

  const generateChecklist = async (stageIndex: number) => {
    if (!workspacePath || !selected) return;
    try {
      setGenerating(true);
      setError(null);
      const updated = await invoke<Workflow>("generate_stage_checklist", {
        workspacePath,
        name: selected.name,
        stageIndex,
        provider,
      });
      setSelected(updated);
      setWorkflows((prev) => prev.map((w) => (w.name === updated.name ? updated : w)));
    } catch (e) {
      setError(String(e));
    } finally {
      setGenerating(false);
    }
  };

  const stage = selected?.stages[activeStage];
  const stageChecked = stage?.checklist.filter((c) => c.done).length ?? 0;
  const stageTotal = stage?.checklist.length ?? 0;
  const canAdvance = selected && activeStage === selected.current_stage && stageTotal > 0 && stageChecked / stageTotal >= 0.8;

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", background: "var(--bg-primary)", color: "var(--text-primary)", fontSize: "13px" }}>
      {/* Header */}
      <div style={{ display: "flex", alignItems: "center", gap: "8px", padding: "10px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)" }}>
        <span style={{ fontWeight: 600 }}>🏗️ Workflow</span>
        {selected && view === "detail" && (
          <button
            onClick={() => { setView("list"); setSelected(null); }}
            style={{ padding: "3px 8px", fontSize: "10px", background: "none", border: "1px solid var(--border-color)", borderRadius: "4px", cursor: "pointer", color: "var(--text-secondary)" }}
          >
            ← Back
          </button>
        )}
        <div style={{ flex: 1 }} />
        <button
          onClick={() => setShowNewForm((f) => !f)}
          style={{ padding: "4px 10px", fontSize: "11px", background: "var(--accent-blue, #007acc)", color: "#fff", border: "none", borderRadius: "4px", cursor: "pointer" }}
        >
          + New Workflow
        </button>
        <button
          onClick={loadWorkflows}
          style={{ padding: "4px 8px", fontSize: "11px", background: "none", border: "1px solid var(--border-color)", borderRadius: "4px", cursor: "pointer", color: "var(--text-secondary)" }}
        >
          ↺
        </button>
      </div>

      {/* New workflow form */}
      {showNewForm && (
        <div style={{ padding: "12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", flexDirection: "column", gap: "8px" }}>
          <input
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            placeholder="Workflow name (e.g. my_todo_app)"
            style={{ padding: "6px 8px", background: "var(--bg-input, #1e1e1e)", border: "1px solid var(--border-color)", borderRadius: "4px", color: "var(--text-primary)", fontSize: "12px" }}
          />
          <textarea
            value={newDesc}
            onChange={(e) => setNewDesc(e.target.value)}
            placeholder="Describe the application you want to build..."
            rows={3}
            style={{ padding: "6px 8px", background: "var(--bg-input, #1e1e1e)", border: "1px solid var(--border-color)", borderRadius: "4px", color: "var(--text-primary)", fontSize: "12px", resize: "vertical" }}
          />
          <div style={{ display: "flex", gap: "8px" }}>
            <button
              onClick={createWorkflow}
              disabled={loading || !newName.trim() || !newDesc.trim()}
              style={{ flex: 1, padding: "6px", background: "var(--accent-blue, #007acc)", color: "#fff", border: "none", borderRadius: "4px", cursor: "pointer", opacity: loading ? 0.6 : 1 }}
            >
              {loading ? "Creating..." : "Create Workflow"}
            </button>
            <button
              onClick={() => setShowNewForm(false)}
              style={{ padding: "6px 12px", background: "none", border: "1px solid var(--border-color)", borderRadius: "4px", cursor: "pointer", color: "var(--text-secondary)" }}
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {/* Error */}
      {error && (
        <div style={{ padding: "8px 12px", background: "#ff4d4f22", color: "#ff4d4f", fontSize: "12px", display: "flex", alignItems: "center", gap: "6px" }}>
          <span>{error}</span>
          <button onClick={() => setError(null)} style={{ marginLeft: "auto", background: "none", border: "none", cursor: "pointer", color: "#ff4d4f" }}>
            ✕
          </button>
        </div>
      )}

      <div style={{ flex: 1, overflow: "auto" }}>
        {/* Workflow List */}
        {view === "list" && (
          <div style={{ padding: "8px" }}>
            {loading && <div style={{ color: "var(--text-secondary)", padding: "20px", textAlign: "center" }}>Loading...</div>}
            {!loading && workflows.length === 0 && (
              <div style={{ color: "var(--text-secondary)", padding: "24px", textAlign: "center" }}>
                <div style={{ fontSize: "32px", marginBottom: "8px" }}>🏗️</div>
                <div>No workflows yet.</div>
                <div style={{ fontSize: "11px", marginTop: "4px" }}>Create one to start building with the Code Complete methodology.</div>
              </div>
            )}
            {workflows.map((w) => (
              <div
                key={w.name}
                onClick={() => openWorkflow(w.name)}
                style={{
                  padding: "10px 12px", marginBottom: "6px", borderRadius: "6px",
                  background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
                  cursor: "pointer", transition: "background 0.15s",
                }}
                onMouseEnter={(e) => (e.currentTarget.style.background = "var(--bg-hover, #2a2d2e)")}
                onMouseLeave={(e) => (e.currentTarget.style.background = "var(--bg-secondary)")}
              >
                <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                  <span style={{ fontSize: "16px" }}>{STAGE_ICONS[w.current_stage] ?? "🏗️"}</span>
                  <span style={{ fontWeight: 600 }}>{w.name.replace(/_/g, " ")}</span>
                  <span style={{ marginLeft: "auto", fontSize: "10px", color: "var(--text-secondary)" }}>
                    Stage {w.current_stage + 1}/8
                  </span>
                </div>
                {w.description && (
                  <div style={{ color: "var(--text-secondary)", fontSize: "11px", marginTop: "4px", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                    {w.description}
                  </div>
                )}
                <div style={{ marginTop: "8px" }}>
                  <div style={{ display: "flex", justifyContent: "space-between", fontSize: "10px", color: "var(--text-secondary)", marginBottom: "3px" }}>
                    <span>{w.stages[w.current_stage]?.label}</span>
                    <span>{Math.round(w.overall_progress)}%</span>
                  </div>
                  <div style={{ height: "3px", background: "var(--border-color)", borderRadius: "2px" }}>
                    <div style={{ width: `${w.overall_progress}%`, height: "100%", background: w.overall_progress === 100 ? "#4caf50" : "var(--accent-blue, #007acc)", borderRadius: "2px", transition: "width 0.3s" }} />
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}

        {/* Workflow Detail */}
        {view === "detail" && selected && (
          <div style={{ padding: "12px", display: "flex", flexDirection: "column", gap: "12px" }}>
            {/* Workflow header */}
            <div>
              <div style={{ display: "flex", alignItems: "center", gap: "8px", flexWrap: "wrap" }}>
                <h3 style={{ margin: 0, fontSize: "15px" }}>{selected.name.replace(/_/g, " ")}</h3>
                <span style={{ padding: "2px 8px", borderRadius: "10px", fontSize: "11px", background: "#007acc33", color: "#007acc" }}>
                  {Math.round(selected.overall_progress)}% complete
                </span>
              </div>
              {selected.description && (
                <div style={{ color: "var(--text-secondary)", fontSize: "11px", marginTop: "4px" }}>{selected.description}</div>
              )}
            </div>

            {/* Pipeline visualization */}
            <div style={{ display: "flex", alignItems: "center", gap: "2px", padding: "8px 0", overflowX: "auto" }}>
              {selected.stages.map((s, i) => {
                const isCurrent = i === selected.current_stage;
                const isActive = i === activeStage;
                const bg = s.status === "complete" ? "#4caf50"
                  : s.status === "in-progress" ? "#ff9800"
                  : s.status === "skipped" ? "#9e9e9e"
                  : "var(--bg-secondary)";
                const border = isActive ? "2px solid var(--accent-blue, #007acc)" : `2px solid ${s.status === "not-started" ? "var(--border-color)" : bg}`;
                return (
                  <div key={i} style={{ display: "flex", alignItems: "center" }}>
                    <div
                      onClick={() => setActiveStage(i)}
                      style={{
                        width: "32px", height: "32px", borderRadius: "50%", display: "flex", alignItems: "center", justifyContent: "center",
                        background: s.status === "not-started" ? "var(--bg-secondary)" : bg + "33",
                        border, cursor: "pointer", fontSize: "14px", position: "relative",
                        transition: "all 0.15s",
                      }}
                      title={`${s.label} (${s.status})`}
                    >
                      {STAGE_ICONS[i]}
                      {isCurrent && (
                        <div style={{ position: "absolute", bottom: "-2px", right: "-2px", width: "8px", height: "8px", borderRadius: "50%", background: "#ff9800", border: "1px solid var(--bg-primary)" }} />
                      )}
                    </div>
                    {i < 7 && (
                      <div style={{ width: "12px", height: "2px", background: s.status === "complete" ? "#4caf50" : "var(--border-color)" }} />
                    )}
                  </div>
                );
              })}
            </div>

            {/* Stage detail */}
            {stage && (
              <div style={{ background: "var(--bg-secondary)", borderRadius: "6px", padding: "12px", borderLeft: `3px solid ${STATUS_COLORS[stage.status] || "#666"}` }}>
                <div style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "8px" }}>
                  <span style={{ fontSize: "16px" }}>{STAGE_ICONS[activeStage]}</span>
                  <span style={{ fontWeight: 600, fontSize: "13px" }}>{stage.label}</span>
                  <span style={{ padding: "2px 6px", borderRadius: "10px", fontSize: "10px", background: (STATUS_COLORS[stage.status] || "#666") + "33", color: STATUS_COLORS[stage.status] || "#666" }}>
                    {stage.status}
                  </span>
                  {stageTotal > 0 && (
                    <span style={{ fontSize: "10px", color: "var(--text-secondary)", marginLeft: "auto" }}>
                      {stageChecked}/{stageTotal} items
                    </span>
                  )}
                </div>

                {/* Stage progress bar */}
                {stageTotal > 0 && (
                  <div style={{ height: "3px", background: "var(--border-color)", borderRadius: "2px", marginBottom: "10px" }}>
                    <div style={{ width: `${stageTotal > 0 ? (stageChecked / stageTotal) * 100 : 0}%`, height: "100%", background: stageChecked === stageTotal ? "#4caf50" : "#ff9800", borderRadius: "2px", transition: "width 0.3s" }} />
                  </div>
                )}

                {/* Checklist */}
                {stage.checklist.length === 0 && (
                  <div style={{ color: "var(--text-secondary)", fontSize: "12px", marginBottom: "8px" }}>
                    No checklist items yet. Generate one with AI.
                  </div>
                )}
                {stage.checklist.map((item) => (
                  <div
                    key={item.id}
                    onClick={() => toggleItem(activeStage, item.id)}
                    style={{
                      display: "flex", alignItems: "flex-start", gap: "10px",
                      padding: "6px 8px", marginBottom: "3px", borderRadius: "4px",
                      background: item.done ? "rgba(76,175,80,0.08)" : "transparent",
                      cursor: "pointer", opacity: item.done ? 0.75 : 1,
                    }}
                  >
                    <div style={{
                      width: "16px", height: "16px", borderRadius: "3px", flexShrink: 0, marginTop: "1px",
                      border: item.done ? "none" : "2px solid var(--border-color)",
                      background: item.done ? "#4caf50" : "transparent",
                      display: "flex", alignItems: "center", justifyContent: "center",
                    }}>
                      {item.done && <span style={{ color: "#fff", fontSize: "10px" }}>✓</span>}
                    </div>
                    <div style={{ fontSize: "12px", textDecoration: item.done ? "line-through" : "none" }}>{item.description}</div>
                  </div>
                ))}

                {/* Action buttons */}
                <div style={{ display: "flex", gap: "8px", marginTop: "10px" }}>
                  <button
                    onClick={() => generateChecklist(activeStage)}
                    disabled={generating}
                    style={{
                      padding: "5px 12px", fontSize: "11px",
                      background: "var(--accent-blue, #007acc)", color: "#fff",
                      border: "none", borderRadius: "4px", cursor: "pointer",
                      opacity: generating ? 0.6 : 1,
                    }}
                  >
                    {generating ? "Generating..." : "Generate Checklist"}
                  </button>
                  {canAdvance && activeStage === selected.current_stage && (
                    <button
                      onClick={advanceStage}
                      style={{
                        padding: "5px 12px", fontSize: "11px",
                        background: "#4caf50", color: "#fff",
                        border: "none", borderRadius: "4px", cursor: "pointer",
                      }}
                    >
                      Advance to Next Stage →
                    </button>
                  )}
                </div>
              </div>
            )}

            {/* Stage body / notes */}
            {stage?.body && (
              <details style={{ background: "var(--bg-secondary)", borderRadius: "6px", padding: "10px 12px" }}>
                <summary style={{ fontSize: "12px", fontWeight: 600, cursor: "pointer", color: "var(--text-secondary)" }}>
                  Notes
                </summary>
                <pre style={{ marginTop: "8px", fontSize: "11px", whiteSpace: "pre-wrap", wordBreak: "break-word", color: "var(--text-primary)", lineHeight: 1.5 }}>
                  {stage.body}
                </pre>
              </details>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
