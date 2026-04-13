import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
// lucide-react icons not needed

interface SteeringFile {
 filename: string;
 name: string;
 scope_label: string | null;
 content: string;
}

interface SteeringPanelProps {
 workspaceRoot?: string;
}

const SCOPE_OPTIONS = [
 { value: "workspace", label: "Workspace (.vibecli/steering/)" },
 { value: "global", label: "Global (~/.vibecli/steering/)" },
];

const TEMPLATES = [
 {
 label: "Architecture",
 filename: "architecture.md",
 content:
 "---\nname: architecture\nscope: project\n---\n\n# Project Architecture\n\nDescribe the high-level architecture here. This will be injected into every agent prompt.\n\n- Framework: \n- State management: \n- Key directories: \n",
 },
 {
 label: "Code Style",
 filename: "code-style.md",
 content:
 "---\nname: code-style\nscope: project\n---\n\n# Code Style Guidelines\n\n- Language: \n- Formatting: \n- Naming conventions: \n- Anti-patterns to avoid: \n",
 },
 {
 label: "Tech Stack",
 filename: "tech-stack.md",
 content:
 "---\nname: tech-stack\nscope: project\n---\n\n# Technology Stack\n\nAlways use these technologies when generating code for this project:\n\n- Frontend: \n- Backend: \n- Database: \n- Testing: \n",
 },
];

export default function SteeringPanel({ workspaceRoot }: SteeringPanelProps) {
 const [scope, setScope] = useState<"workspace" | "global">("workspace");
 const [files, setFiles] = useState<SteeringFile[]>([]);
 const [selected, setSelected] = useState<SteeringFile | null>(null);
 const [editContent, setEditContent] = useState("");
 const [editFilename, setEditFilename] = useState("");
 const [isNew, setIsNew] = useState(false);
 const [saving, setSaving] = useState(false);
 const [error, setError] = useState<string | null>(null);
 const [showTemplates, setShowTemplates] = useState(false);
 const [pendingDelete, setPendingDelete] = useState<string | null>(null);

 const load = useCallback(async () => {
 try {
 const result = await invoke<SteeringFile[]>("get_steering_files", {
 scope,
 workspaceRoot: workspaceRoot || null,
 });
 setFiles(result);
 setError(null);
 } catch (e) {
 setError(String(e));
 }
 }, [scope, workspaceRoot]);

 useEffect(() => {
 setError(null);
 load();
 setSelected(null);
 setIsNew(false);
 }, [load]);

 function selectFile(f: SteeringFile) {
 setSelected(f);
 setEditContent(f.content);
 setEditFilename(f.filename);
 setIsNew(false);
 setShowTemplates(false);
 }

 function startNew() {
 setSelected(null);
 setEditFilename("");
 setEditContent("---\nname: \nscope: project\n---\n\n");
 setIsNew(true);
 setShowTemplates(false);
 }

 function applyTemplate(tpl: (typeof TEMPLATES)[number]) {
 setEditFilename(tpl.filename);
 setEditContent(tpl.content);
 setIsNew(true);
 setSelected(null);
 setShowTemplates(false);
 }

 async function save() {
 if (!editFilename.trim()) {
 setError("Filename is required");
 return;
 }
 setSaving(true);
 setError(null);
 try {
 await invoke("save_steering_file", {
 scope,
 workspaceRoot: workspaceRoot || null,
 filename: editFilename.trim(),
 content: editContent,
 });
 await load();
 setIsNew(false);
 } catch (e) {
 setError(String(e));
 } finally {
 setSaving(false);
 }
 }

 async function deleteFile(filename: string) {
 if (pendingDelete !== filename) { setPendingDelete(filename); return; }
 setPendingDelete(null);
 try {
 await invoke("delete_steering_file", {
 scope,
 workspaceRoot: workspaceRoot || null,
 filename,
 });
 if (selected?.filename === filename) {
 setSelected(null);
 setIsNew(false);
 }
 await load();
 } catch (e) {
 setError(String(e));
 }
 }

 const editing = isNew || selected !== null;

 return (
 <div className="panel-container">
 {/* Header */}
 <div className="panel-header">
 <h3>Steering Files</h3>
 <select
 value={scope}
 onChange={(e) => setScope(e.target.value as "workspace" | "global")}
 className="panel-select"
 >
 {SCOPE_OPTIONS.map((o) => (
 <option key={o.value} value={o.value}>{o.label}</option>
 ))}
 </select>
 <button onClick={startNew} className="panel-btn panel-btn-secondary">+ New</button>
 <button onClick={() => setShowTemplates((v) => !v)} className="panel-btn panel-btn-secondary">Templates</button>
 <button onClick={load} className="panel-btn panel-btn-secondary">↻ Refresh</button>
 </div>

 {error && (
 <div className="panel-error">
 {error}
 </div>
 )}

 {/* Templates dropdown */}
 {showTemplates && (
 <div style={{ background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: 6, padding: 8, display: "flex", gap: 6, flexWrap: "wrap" }}>
 <span style={{ color: "var(--text-secondary)", fontSize: 11, width: "100%" }}>Choose a template to get started:</span>
 {TEMPLATES.map((tpl) => (
 <button key={tpl.label} onClick={() => applyTemplate(tpl)} className="panel-btn panel-btn-secondary" style={{ padding: "4px 10px" }}>
 {tpl.label}
 </button>
 ))}
 </div>
 )}

 <div className="panel-body" style={{ flexDirection: "row", gap: 8 }}>
 {/* File list */}
 <div style={{ width: 160, flexShrink: 0, overflowY: "auto", background: "var(--bg-tertiary)", borderRadius: 6, padding: 4 }}>
 {files.length === 0 && (
 <div style={{ color: "var(--text-secondary)", fontSize: 12, padding: 8, textAlign: "center" }}>
 No steering files.<br />Click "+ New" to create one.
 </div>
 )}
 {files.map((f) => (
 <div
 key={f.filename}
 onClick={() => selectFile(f)}
 style={{
 display: "flex",
 alignItems: "center",
 justifyContent: "space-between",
 padding: "5px 6px",
 borderRadius: 4,
 cursor: "pointer",
 background: selected?.filename === f.filename ? "var(--bg-secondary)" : "transparent",
 marginBottom: 2,
 }}
 >
 <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", flex: 1 }} title={f.filename}>
 {f.name || f.filename}
 </span>
 {pendingDelete === f.filename ? (
 <span style={{ display: "flex", gap: 2 }} onClick={(e) => e.stopPropagation()}>
 <button
 onClick={() => deleteFile(f.filename)}
 title="Confirm delete"
 style={{ background: "none", border: "none", color: "var(--text-danger)", cursor: "pointer", fontSize: 10, padding: "0 2px", lineHeight: 1, fontWeight: 600 }}
 >
 Del
 </button>
 <button
 onClick={() => setPendingDelete(null)}
 title="Cancel"
 style={{ background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer", fontSize: 10, padding: "0 2px", lineHeight: 1 }}
 >
 ✕
 </button>
 </span>
 ) : (
 <button
 onClick={(e) => { e.stopPropagation(); deleteFile(f.filename); }}
 title="Delete"
 style={{ background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer", fontSize: 12, padding: "0 2px", lineHeight: 1 }}
 >
 ✕
 </button>
 )}
 </div>
 ))}
 </div>

 {/* Editor */}
 <div style={{ flex: 1, display: "flex", flexDirection: "column", gap: 6, minHeight: 0 }}>
 {!editing ? (
 <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center", color: "var(--text-secondary)", fontSize: 13 }}>
 Select a steering file or click "+ New"
 </div>
 ) : (
 <>
 <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
 <input
 type="text"
 placeholder="filename.md"
 value={editFilename}
 onChange={(e) => setEditFilename(e.target.value)}
 disabled={!isNew}
 className="panel-input"
 style={{ flex: 1 }}
 />
 <button onClick={save} disabled={saving} className="panel-btn panel-btn-primary" style={{ background: saving ? "var(--bg-secondary)" : undefined, color: saving ? "var(--text-info)" : undefined }}>
 {saving ? "Saving…" : "Save"}
 </button>
 </div>

 <textarea
 value={editContent}
 onChange={(e) => setEditContent(e.target.value)}
 spellCheck={false}
 className="panel-input panel-textarea"
 style={{
 flex: 1,
 resize: "none",
 fontSize: 13,
 fontFamily: "var(--font-mono)",
 lineHeight: 1.6,
 minHeight: 200,
 }}
 />

 <div style={{ color: "var(--text-secondary)", fontSize: 11 }}>
 Steering files inject into every agent prompt. Use YAML front-matter:{" "}
 <code style={{ background: "var(--bg-tertiary)", padding: "1px 4px", borderRadius: 3 }}>--- name: my-doc scope: project ---</code>
 </div>
 </>
 )}
 </div>
 </div>
 </div>
 );
}
