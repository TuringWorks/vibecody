import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2 } from "lucide-react";

interface SpecTask {
 id: number;
 description: string;
 done: boolean;
}

interface Spec {
 name: string;
 status: "draft" | "approved" | "in-progress" | "done";
 requirements: string;
 tasks: SpecTask[];
 body: string;
 source: string;
}

interface SpecPanelProps {
 workspacePath: string | null;
 provider?: string;
}

const STATUS_COLORS: Record<string, string> = {
 draft: "var(--text-secondary)",
 approved: "var(--success-color)",
 "in-progress": "var(--warning-color)",
 done: "var(--info-color)",
};

const STATUS_ICONS: Record<string, string> = {
 draft: "",
 approved: "",
 "in-progress": "",
 done: "",
};

export function SpecPanel({ workspacePath, provider = "ollama" }: SpecPanelProps) {
 const [specs, setSpecs] = useState<Spec[]>([]);
 const [selectedSpec, setSelectedSpec] = useState<Spec | null>(null);
 const [activeTab, setActiveTab] = useState<"list" | "editor">("list");
 const [loading, setLoading] = useState(false);
 const [generating, setGenerating] = useState(false);
 const [error, setError] = useState<string | null>(null);

 // New spec form
 const [newSpecName, setNewSpecName] = useState("");
 const [newSpecRequirements, setNewSpecRequirements] = useState("");
 const [showNewForm, setShowNewForm] = useState(false);

 const loadSpecs = useCallback(async () => {
 if (!workspacePath) return;
 try {
 setLoading(true);
 const list = await invoke<Spec[]>("list_specs", { workspacePath });
 setSpecs(list);
 } catch (e) {
 setError(String(e));
 } finally {
 setLoading(false);
 }
 }, [workspacePath]);

 useEffect(() => {
 loadSpecs();
 }, [loadSpecs]);

 const openSpec = async (name: string) => {
 if (!workspacePath) return;
 try {
 const spec = await invoke<Spec>("get_spec", { workspacePath, name });
 setSelectedSpec(spec);
 setActiveTab("editor");
 } catch (e) {
 setError(String(e));
 }
 };

 const generateSpec = async () => {
 if (!workspacePath || !newSpecName.trim() || !newSpecRequirements.trim()) return;
 try {
 setGenerating(true);
 setError(null);
 const spec = await invoke<Spec>("generate_spec", {
 workspacePath,
 name: newSpecName.trim().replace(/\s+/g, "_").toLowerCase(),
 requirements: newSpecRequirements.trim(),
 provider,
 });
 setSpecs(prev => [...prev, spec]);
 setSelectedSpec(spec);
 setActiveTab("editor");
 setShowNewForm(false);
 setNewSpecName("");
 setNewSpecRequirements("");
 } catch (e) {
 setError(String(e));
 } finally {
 setGenerating(false);
 }
 };

 const toggleTask = async (taskId: number) => {
 if (!workspacePath || !selectedSpec) return;
 try {
 const updated = await invoke<Spec>("update_spec_task", {
 workspacePath,
 name: selectedSpec.name,
 taskId,
 done: !selectedSpec.tasks.find(t => t.id === taskId)?.done,
 });
 setSelectedSpec(updated);
 setSpecs(prev => prev.map(s => s.name === updated.name ? updated : s));
 } catch (e) {
 setError(String(e));
 }
 };

 const runSpec = async () => {
 if (!workspacePath || !selectedSpec) return;
 try {
 setLoading(true);
 // get the task prompt from the spec
 const taskPrompt = await invoke<string>("run_spec", { workspacePath, name: selectedSpec.name });
 if (!taskPrompt) {
 setError("All tasks are already complete!");
 return;
 }
 // kick off the agent with the spec task
 await invoke("start_agent_task", { task: taskPrompt, approvalPolicy: "auto-edit", provider });
 } catch (e) {
 setError(String(e));
 } finally {
 setLoading(false);
 }
 };

 const pendingCount = selectedSpec?.tasks.filter(t => !t.done).length ?? 0;
 const doneCount = selectedSpec?.tasks.filter(t => t.done).length ?? 0;

 return (
 <div className="panel-container">
 {/* Header */}
 <div className="panel-header">
 <h3>Specs</h3>
 <div style={{ flex: 1 }} />
 <button
 onClick={() => setShowNewForm(f => !f)}
 className="panel-btn panel-btn-primary panel-btn-sm"
 >
 + New Spec
 </button>
 <button
 onClick={loadSpecs}
 className="panel-btn panel-btn-secondary panel-btn-sm"
 aria-label="Refresh"
 >
 ↺
 </button>
 </div>

 {/* New spec form */}
 {showNewForm && (
 <div className="panel-card" style={{ margin: "8px 12px", display: "flex", flexDirection: "column", gap: "8px" }}>
 <input
 value={newSpecName}
 onChange={e => setNewSpecName(e.target.value)}
 placeholder="Spec name (e.g. dark_mode)"
 className="panel-input panel-input-full"
 />
 <textarea
 value={newSpecRequirements}
 onChange={e => setNewSpecRequirements(e.target.value)}
 placeholder="Describe the requirements in natural language..."
 rows={4}
 className="panel-input panel-textarea panel-input-full"
 style={{ resize: "vertical" }}
 />
 <div style={{ display: "flex", gap: "8px" }}>
 <button
 onClick={generateSpec}
 disabled={generating || !newSpecName.trim() || !newSpecRequirements.trim()}
 className="panel-btn panel-btn-primary"
 style={{ flex: 1 }}
 >
 {generating ? <><Loader2 size={13} className="spin" /> Generating...</> : "Generate Spec"}
 </button>
 <button
 onClick={() => setShowNewForm(false)}
 className="panel-btn panel-btn-secondary"
 >
 Cancel
 </button>
 </div>
 </div>
 )}

 {/* Error */}
 {error && (
 <div className="panel-error">
 <span>{error}</span>
 <button onClick={() => setError(null)}>✕</button>
 </div>
 )}

 {/* Sub-tabs */}
 <div className="panel-tab-bar" style={{ padding: "0 16px" }} role="tablist">
 {(["list", "editor"] as const).map(tab => (
 <button
 key={tab}
 onClick={() => setActiveTab(tab)}
 className={`panel-tab${activeTab === tab ? " active" : ""}`}
 style={{ flex: 1 }}
 role="tab"
 aria-selected={activeTab === tab}
 >
 {tab === "list" ? ` All Specs (${specs.length})` : ` ${selectedSpec?.name ?? "Select a spec"}`}
 </button>
 ))}
 </div>

 <div className="panel-body" style={{ overflow: "auto" }} role="tabpanel">
 {/* Spec List */}
 {activeTab === "list" && (
 <div style={{ padding: "8px" }}>
 {loading && <div className="panel-loading">Loading...</div>}
 {!loading && specs.length === 0 && (
 <div className="panel-empty">No specs yet. Create one to get started.</div>
 )}
 {specs.map(spec => {
 const done = spec.tasks.filter(t => t.done).length;
 const total = spec.tasks.length;
 const progress = total > 0 ? (done / total) * 100 : 0;
 return (
 <div
 key={spec.name}
 onClick={() => openSpec(spec.name)}
 className="panel-card"
 style={{ marginBottom: "8px", cursor: "pointer", transition: "background 0.15s" }}
 onMouseEnter={e => (e.currentTarget.style.background = "var(--bg-hover)")}
 onMouseLeave={e => (e.currentTarget.style.background = "")}
 role="button"
 tabIndex={0}
 onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); openSpec(spec.name); } }}
 >
 <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
 <span style={{ fontSize: "16px" }}>{STATUS_ICONS[spec.status] ?? ""}</span>
 <span style={{ fontWeight: 600 }}>{spec.name.replace(/_/g, " ")}</span>
 <span style={{ marginLeft: "auto", padding: "2px 8px", borderRadius: "var(--radius-md)", fontSize: "var(--font-size-xs)", background: STATUS_COLORS[spec.status] + "33", color: STATUS_COLORS[spec.status] }}>
 {spec.status}
 </span>
 </div>
 {spec.requirements && (
 <div style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-sm)", marginTop: "4px", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
 {spec.requirements}
 </div>
 )}
 {total > 0 && (
 <div style={{ marginTop: "8px" }}>
 <div style={{ display: "flex", justifyContent: "space-between", fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", marginBottom: "3px" }}>
 <span>{done}/{total} tasks</span>
 <span>{Math.round(progress)}%</span>
 </div>
 <div style={{ height: "3px", background: "var(--border-color)", borderRadius: "2px" }}>
 <div style={{ width: `${progress}%`, height: "100%", background: progress === 100 ? "var(--success-color)" : "var(--accent-color)", borderRadius: "2px", transition: "width 0.3s" }} />
 </div>
 </div>
 )}
 </div>
 );
 })}
 </div>
 )}

 {/* Spec Editor */}
 {activeTab === "editor" && selectedSpec && (
 <div style={{ padding: "12px", display: "flex", flexDirection: "column", gap: "12px" }}>
 {/* Spec header */}
 <div style={{ display: "flex", alignItems: "center", gap: "8px", flexWrap: "wrap" }}>
 <h3 style={{ margin: 0, fontSize: "var(--font-size-xl)" }}>{selectedSpec.name.replace(/_/g, " ")}</h3>
 <span style={{ padding: "2px 8px", borderRadius: "var(--radius-md)", fontSize: "var(--font-size-sm)", background: STATUS_COLORS[selectedSpec.status] + "33", color: STATUS_COLORS[selectedSpec.status] }}>
 {selectedSpec.status}
 </span>
 <div style={{ flex: 1 }} />
 <button className="panel-btn"
 onClick={runSpec}
 disabled={loading || pendingCount === 0}
 style={{
 padding: "4px 12px", fontSize: "var(--font-size-sm)", background: pendingCount > 0 ? "var(--accent-color)" : "var(--bg-secondary)",
 color: pendingCount > 0 ? "var(--text-primary)" : "var(--text-secondary)", border: "none", borderRadius: "var(--radius-xs-plus)",
 cursor: pendingCount > 0 ? "pointer" : "not-allowed", opacity: loading ? 0.6 : 1,
 }}
 title={pendingCount === 0 ? "All tasks complete" : `Run agent on ${pendingCount} pending tasks`}
 >
 {loading ? <><Loader2 size={13} className="spin" /> Running...</> : ` Run Agent (${pendingCount} pending)`}
 </button>
 </div>

 {/* Requirements */}
 {selectedSpec.requirements && (
 <div style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", padding: "12px 12px", borderLeft: "3px solid var(--accent-color)" }}>
 <div style={{ fontSize: "var(--font-size-xs)", fontWeight: 600, color: "var(--text-secondary)", marginBottom: "4px", textTransform: "uppercase", letterSpacing: "0.5px" }}>Requirements</div>
 <div style={{ fontSize: "var(--font-size-base)" }}>{selectedSpec.requirements}</div>
 </div>
 )}

 {/* Task list */}
 <div>
 <div style={{ fontSize: "var(--font-size-base)", fontWeight: 600, marginBottom: "8px", display: "flex", alignItems: "center", gap: "8px" }}>
 Tasks
 <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", fontWeight: 400 }}>
 {doneCount}/{selectedSpec.tasks.length} complete
 </span>
 </div>
 {selectedSpec.tasks.length === 0 && (
 <div style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-base)" }}>No tasks generated yet.</div>
 )}
 {selectedSpec.tasks.map(task => (
 <div
 key={task.id}
 onClick={() => toggleTask(task.id)}
 style={{
 display: "flex", alignItems: "flex-start", gap: "12px",
 padding: "8px 12px", marginBottom: "4px", borderRadius: "5px",
 background: task.done ? "var(--bg-secondary)" : "transparent",
 border: "1px solid var(--border-color)",
 cursor: "pointer", opacity: task.done ? 0.7 : 1,
 }}
 role="button"
 tabIndex={0}
 onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggleTask(task.id); } }}
 >
 <div style={{
 width: "16px", height: "16px", borderRadius: "3px", flexShrink: 0, marginTop: "1px",
 border: task.done ? "none" : "2px solid var(--border-color)",
 background: task.done ? "var(--success-color)" : "transparent",
 display: "flex", alignItems: "center", justifyContent: "center",
 }}>
 {task.done && <span style={{ color: "var(--text-primary)", fontSize: "var(--font-size-xs)" }}>✓</span>}
 </div>
 <div style={{ flex: 1 }}>
 <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: "2px" }}>Task {task.id}</div>
 <div style={{ fontSize: "var(--font-size-base)", textDecoration: task.done ? "line-through" : "none" }}>{task.description}</div>
 </div>
 </div>
 ))}
 </div>

 {/* Body (collapsible) */}
 {selectedSpec.body && (
 <details style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", padding: "12px 12px" }}>
 <summary style={{ fontSize: "var(--font-size-base)", fontWeight: 600, cursor: "pointer", color: "var(--text-secondary)" }}>
 Full Spec Document
 </summary>
 <pre style={{ marginTop: "8px", fontSize: "var(--font-size-sm)", whiteSpace: "pre-wrap", wordBreak: "break-word", color: "var(--text-primary)", lineHeight: 1.5 }}>
 {selectedSpec.body}
 </pre>
 </details>
 )}
 </div>
 )}

 {activeTab === "editor" && !selectedSpec && (
 <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "200px", color: "var(--text-secondary)" }}>
 Select a spec from the list
 </div>
 )}
 </div>
 </div>
 );
}
