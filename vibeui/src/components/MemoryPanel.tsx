import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useToast } from "../hooks/useToast";
import { Toaster } from "./Toaster";
import { Pin } from "lucide-react";

interface MemoryPanelProps {
 workspacePath?: string | null;
}

type RulesTab = "workspace" | "global" | "directory" | "auto";

interface MemoryFact {
 id: string;
 fact: string;
 confidence: number;
 tags: string[];
 pinned: boolean;
 session_id: string | null;
}

interface RuleFileMeta {
 filename: string;
 name: string;
 path_pattern: string | null;
}

// -- Directory Rules Sub-panel ------------------------------------------------

function DirRulesTab({ workspacePath }: { workspacePath?: string | null }) {
 const [scope, setScope] = useState<"workspace" | "global">("workspace");
 const [files, setFiles] = useState<RuleFileMeta[]>([]);
 const [selected, setSelected] = useState<string | null>(null);
 const [content, setContent] = useState("");
 const [loading, setLoading] = useState(false);
 const [saving, setSaving] = useState(false);
 const [saved, setSaved] = useState(false);
 const [error, setError] = useState<string | null>(null);
 const [creating, setCreating] = useState(false);
 const [newName, setNewName] = useState("");
 const [newPattern, setNewPattern] = useState("");
 const [confirmDelete, setConfirmDelete] = useState<string | null>(null);

 useEffect(() => {
 loadFiles();
 setSelected(null);
 setContent("");
 setError(null);
 // eslint-disable-next-line react-hooks/exhaustive-deps
 }, [scope, workspacePath]);

 async function loadFiles() {
 setLoading(true);
 setError(null);
 try {
 const list = await invoke<RuleFileMeta[]>("list_rule_files", { scope });
 setFiles(list);
 } catch (e) {
 setError(String(e));
 } finally {
 setLoading(false);
 }
 }

 async function selectFile(filename: string) {
 setSelected(filename);
 setSaved(false);
 try {
 const text = await invoke<string>("get_rule_file", { scope, filename });
 setContent(text);
 } catch (e) {
 setError(String(e));
 }
 }

 async function saveFile() {
 if (!selected) return;
 setSaving(true);
 setSaved(false);
 setError(null);
 try {
 await invoke("save_rule_file", { scope, filename: selected, content });
 setSaved(true);
 setTimeout(() => setSaved(false), 2000);
 await loadFiles();
 } catch (e) {
 setError(String(e));
 } finally {
 setSaving(false);
 }
 }

 async function createFile() {
 const rawName = newName.trim();
 if (!rawName) return;
 const filename = rawName.endsWith(".md") ? rawName : `${rawName}.md`;
 const frontmatter = newPattern.trim()
 ? `---\nname: ${rawName.replace(/\.md$/, "")}\npath_pattern: "${newPattern.trim()}"\n---\n\n`
 : `---\nname: ${rawName.replace(/\.md$/, "")}\n---\n\n`;
 setSaving(true);
 setError(null);
 try {
 await invoke("save_rule_file", { scope, filename, content: frontmatter });
 setCreating(false);
 setNewName("");
 setNewPattern("");
 await loadFiles();
 await selectFile(filename);
 } catch (e) {
 setError(String(e));
 } finally {
 setSaving(false);
 }
 }

 async function deleteFile(filename: string) {
 setError(null);
 try {
 await invoke("delete_rule_file", { scope, filename });
 setConfirmDelete(null);
 if (selected === filename) {
 setSelected(null);
 setContent("");
 }
 await loadFiles();
 } catch (e) {
 setError(String(e));
 }
 }

 const dirLabel = scope === "workspace"
 ? "<workspace>/.vibecli/rules/"
 : "~/.vibecli/rules/";

 return (
 <div style={{ display: "flex", flexDirection: "column", flex: 1, minHeight: 0, gap: "8px" }}>
 {/* Scope selector */}
 <div style={{ display: "flex", gap: "4px" }}>
 {(["workspace", "global"] as const).map((s) => (
 <button
 key={s}
 onClick={() => setScope(s)}
 style={{
 padding: "3px 8px",
 fontSize: "var(--font-size-sm)",
 borderRadius: "var(--radius-xs-plus)",
 background: scope === s ? "var(--accent-color)" : "var(--bg-tertiary)",
 color: scope === s ? "white" : "var(--text-primary)",
 border: "1px solid var(--border-color)",
 cursor: "pointer",
 }}
 >
 {s === "workspace" ? "Project" : "Global"}
 </button>
 ))}
 <button
 onClick={() => { setCreating(true); setNewName(""); setNewPattern(""); }}
 style={{
 marginLeft: "auto",
 padding: "3px 8px",
 fontSize: "var(--font-size-sm)",
 background: "var(--bg-tertiary)",
 border: "1px solid var(--border-color)",
 borderRadius: "var(--radius-xs-plus)",
 color: "var(--text-primary)",
 cursor: "pointer",
 }}
 >
 + New Rule
 </button>
 </div>

 {scope === "workspace" && !workspacePath && (
 <div style={{ fontSize: "var(--font-size-base)", color: "var(--warning-color)", padding: "6px", background: "color-mix(in srgb, var(--accent-rose) 10%, transparent)", borderRadius: "var(--radius-xs-plus)" }}>
 Open a folder to manage project rules.
 </div>
 )}

 {error && (
 <div style={{ fontSize: "var(--font-size-base)", color: "var(--error-color)", padding: "6px 8px", background: "color-mix(in srgb, var(--accent-rose) 15%, transparent)", borderRadius: "var(--radius-xs-plus)" }}>
 {error}
 </div>
 )}

 {/* New rule form */}
 {creating && (
 <div style={{ padding: "8px", background: "var(--bg-tertiary)", borderRadius: "var(--radius-xs-plus)", border: "1px solid var(--border-color)", display: "flex", flexDirection: "column", gap: "6px" }}>
 <div style={{ fontSize: "var(--font-size-sm)", fontWeight: 600, color: "var(--text-secondary)" }}>New Rule File</div>
 <input
 autoFocus
 type="text"
 value={newName}
 onChange={(e) => setNewName(e.target.value)}
 onKeyDown={(e) => { if (e.key === "Enter") createFile(); if (e.key === "Escape") setCreating(false); }}
 placeholder="filename (e.g. rust-safety)"
 style={{ padding: "4px 8px", fontSize: "var(--font-size-base)", background: "var(--bg-input, var(--bg-primary))", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", outline: "none" }}
 />
 <input
 type="text"
 value={newPattern}
 onChange={(e) => setNewPattern(e.target.value)}
 placeholder="path_pattern (optional, e.g. **/*.rs)"
 style={{ padding: "4px 8px", fontSize: "var(--font-size-base)", background: "var(--bg-input, var(--bg-primary))", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", outline: "none" }}
 />
 <div style={{ display: "flex", gap: "6px" }}>
 <button onClick={createFile} disabled={!newName.trim() || saving}
 style={{ padding: "4px 10px", fontSize: "var(--font-size-base)", background: "var(--accent-color)", color: "var(--btn-primary-fg)", border: "none", borderRadius: "var(--radius-xs-plus)", cursor: "pointer" }}>
 Create
 </button>
 <button onClick={() => setCreating(false)}
 style={{ padding: "4px 10px", fontSize: "var(--font-size-base)", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", cursor: "pointer" }}>
 Cancel
 </button>
 </div>
 </div>
 )}

 {/* Main two-column layout */}
 <div style={{ display: "flex", gap: "8px", flex: 1, minHeight: 0 }}>
 {/* File list */}
 <div style={{ width: "140px", flexShrink: 0, overflowY: "auto", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", background: "var(--bg-tertiary)" }}>
 {loading && (
 <div style={{ padding: "8px", fontSize: "var(--font-size-base)", color: "var(--text-secondary)", textAlign: "center" }}>…</div>
 )}
 {!loading && files.length === 0 && (
 <div style={{ padding: "8px", fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", textAlign: "center", lineHeight: 1.4 }}>
 No rules yet.<br />Click + New Rule.
 </div>
 )}
 {files.map((f) => (
 <div
 key={f.filename}
 onClick={() => selectFile(f.filename)}
 style={{
 padding: "6px 8px",
 cursor: "pointer",
 background: selected === f.filename ? "var(--accent-color)" : "transparent",
 color: selected === f.filename ? "white" : "var(--text-primary)",
 borderBottom: "1px solid var(--border-color)",
 display: "flex",
 flexDirection: "column",
 gap: "2px",
 }}
 >
 <div style={{ fontSize: "var(--font-size-base)", fontWeight: selected === f.filename ? 600 : 400, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
 {f.name}
 </div>
 {f.path_pattern && (
 <div style={{ fontSize: "var(--font-size-xs)", opacity: 0.7, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
 {f.path_pattern}
 </div>
 )}
 {!f.path_pattern && (
 <div style={{ fontSize: "var(--font-size-xs)", opacity: 0.5 }}>always</div>
 )}
 </div>
 ))}
 </div>

 {/* Editor */}
 {selected ? (
 <div style={{ flex: 1, display: "flex", flexDirection: "column", gap: "6px", minWidth: 0 }}>
 <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", fontFamily: "var(--font-mono)" }}>
 {dirLabel}{selected}
 </div>
 <textarea
 value={content}
 onChange={(e) => setContent(e.target.value)}
 style={{
 flex: 1,
 background: "var(--bg-tertiary)",
 border: "1px solid var(--border-color)",
 color: "var(--text-primary)",
 borderRadius: "var(--radius-xs-plus)",
 padding: "8px",
 fontSize: "var(--font-size-base)",
 fontFamily: "var(--font-mono)",
 resize: "none",
 outline: "none",
 }}
 placeholder="Write your rule content here…"
 />
 <div style={{ display: "flex", gap: "6px" }}>
 <button
 onClick={saveFile}
 disabled={saving}
 style={{ padding: "5px 12px", fontSize: "var(--font-size-base)", background: "var(--accent-color)", color: "var(--btn-primary-fg)", border: "none", borderRadius: "var(--radius-xs-plus)", cursor: "pointer" }}
 >
 {saving ? "Saving…" : saved ? "✓ Saved" : "Save"}
 </button>
 <button
 onClick={() => setConfirmDelete(selected)}
 style={{ padding: "5px 12px", fontSize: "var(--font-size-base)", background: "transparent", border: "1px solid var(--error-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--error-color)", cursor: "pointer", marginLeft: "auto" }}
 >
 Delete
 </button>
 </div>
 </div>
 ) : (
 <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center", fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>
 Select a rule to edit
 </div>
 )}
 </div>

 {/* Dir label */}
 <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
 Files in <code style={{ fontSize: "var(--font-size-xs)" }}>{dirLabel}</code>
 </div>

 {/* Confirm delete modal */}
 {confirmDelete && (
 <div style={{ position: "absolute", inset: 0, background: "rgba(0,0,0,0.5)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 100 }}>
 <div style={{ background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm-alt)", padding: "20px", maxWidth: "300px", width: "90%" }}>
 <div style={{ fontSize: "var(--font-size-md)", fontWeight: 600, marginBottom: "10px" }}>Delete Rule?</div>
 <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: "16px" }}>
 Permanently delete <strong>{confirmDelete}</strong>?
 </div>
 <div style={{ display: "flex", gap: "8px", justifyContent: "flex-end" }}>
 <button onClick={() => setConfirmDelete(null)}
 style={{ padding: "6px 14px", fontSize: "var(--font-size-base)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", cursor: "pointer" }}>
 Cancel
 </button>
 <button onClick={() => deleteFile(confirmDelete)}
 style={{ padding: "6px 14px", fontSize: "var(--font-size-base)", background: "var(--error-color)", border: "none", borderRadius: "var(--radius-xs-plus)", color: "var(--btn-primary-fg)", cursor: "pointer" }}>
 Delete
 </button>
 </div>
 </div>
 </div>
 )}
 </div>
 );
}

// -- AutoFactsTab -------------------------------------------------------------

function AutoFactsTab() {
 const [facts, setFacts] = useState<MemoryFact[]>([]);
 const [loading, setLoading] = useState(false);
 const [error, setError] = useState<string | null>(null);
 const [newFact, setNewFact] = useState("");
 const [newTags, setNewTags] = useState("");
 const [adding, setAdding] = useState(false);
 const [showAdd, setShowAdd] = useState(false);

 const load = useCallback(async () => {
 setLoading(true);
 setError(null);
 try {
 const result = await invoke<MemoryFact[]>("get_auto_memories");
 // Sort: pinned first, then by confidence desc
 result.sort((a, b) => {
 if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
 return b.confidence - a.confidence;
 });
 setFacts(result);
 } catch (e) {
 setError(String(e));
 } finally {
 setLoading(false);
 }
 }, []);

 useEffect(() => { load(); }, [load]);

 async function togglePin(id: string, pinned: boolean) {
 try {
 await invoke("pin_auto_memory", { id, pinned: !pinned });
 await load();
 } catch (e) {
 setError(String(e));
 }
 }

 async function deleteFact(id: string) {
 try {
 await invoke("delete_auto_memory", { id });
 setFacts((prev) => prev.filter((f) => f.id !== id));
 } catch (e) {
 setError(String(e));
 }
 }

 async function addFact() {
 if (!newFact.trim()) return;
 setAdding(true);
 try {
 const tags = newTags.split(",").map((t) => t.trim()).filter(Boolean);
 await invoke("add_auto_memory", { fact: newFact.trim(), tags });
 setNewFact("");
 setNewTags("");
 setShowAdd(false);
 await load();
 } catch (e) {
 setError(String(e));
 } finally {
 setAdding(false);
 }
 }

 function confidenceColor(c: number) {
 if (c >= 0.85) return "var(--success-color)";
 if (c >= 0.65) return "var(--warning-color)";
 return "var(--error-color)";
 }

 return (
 <div style={{ display: "flex", flexDirection: "column", flex: 1, minHeight: 0, gap: 8 }}>
 <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
 <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>
 {facts.length} fact{facts.length !== 1 ? "s" : ""} extracted from sessions
 </span>
 <button
 onClick={() => setShowAdd((v) => !v)}
 style={{ marginLeft: "auto", padding: "3px 8px", fontSize: "var(--font-size-sm)", borderRadius: "var(--radius-xs-plus)", background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", color: "var(--text-primary)", cursor: "pointer" }}
 >
 + Add Fact
 </button>
 <button
 onClick={load}
 style={{ padding: "3px 8px", fontSize: "var(--font-size-sm)", borderRadius: "var(--radius-xs-plus)", background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", color: "var(--text-primary)", cursor: "pointer" }}
 >
 ↻
 </button>
 </div>

 {showAdd && (
 <div style={{ background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", padding: 10, display: "flex", flexDirection: "column", gap: 6 }}>
 <textarea
 autoFocus
 placeholder="Enter a fact to remember across sessions…"
 value={newFact}
 onChange={(e) => setNewFact(e.target.value)}
 rows={2}
 style={{ resize: "none", padding: "4px 8px", fontSize: "var(--font-size-base)", borderRadius: "var(--radius-xs-plus)", background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontFamily: "inherit" }}
 />
 <input
 type="text"
 placeholder="Tags (comma-separated, e.g. rust, testing)"
 value={newTags}
 onChange={(e) => setNewTags(e.target.value)}
 style={{ padding: "4px 8px", fontSize: "var(--font-size-base)", borderRadius: "var(--radius-xs-plus)", background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)" }}
 />
 <div style={{ display: "flex", gap: 6 }}>
 <button
 onClick={addFact}
 disabled={adding || !newFact.trim()}
 style={{ padding: "4px 12px", fontSize: "var(--font-size-base)", borderRadius: "var(--radius-xs-plus)", background: "var(--accent-color)", color: "var(--btn-primary-fg)", border: "none", cursor: "pointer" }}
 >
 {adding ? "Saving…" : "Save"}
 </button>
 <button
 onClick={() => setShowAdd(false)}
 style={{ padding: "4px 12px", fontSize: "var(--font-size-base)", borderRadius: "var(--radius-xs-plus)", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", color: "var(--text-primary)", cursor: "pointer" }}
 >
 Cancel
 </button>
 </div>
 </div>
 )}

 {error && (
 <div style={{ fontSize: "var(--font-size-base)", color: "var(--error-color)", padding: "4px 8px", background: "color-mix(in srgb, var(--accent-rose) 15%, transparent)", borderRadius: "var(--radius-xs-plus)" }}>
 {error}
 </div>
 )}

 <div style={{ flex: 1, overflowY: "auto", display: "flex", flexDirection: "column", gap: 4 }}>
 {loading && <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", padding: 8, textAlign: "center" }}>Loading…</div>}
 {!loading && facts.length === 0 && (
 <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", padding: 16, textAlign: "center", lineHeight: 1.6 }}>
 No auto-extracted memories yet.<br />
 Facts are extracted automatically after agent sessions complete.<br />
 You can also click "+ Add Fact" to add manually.
 </div>
 )}
 {facts.map((f) => (
 <div
 key={f.id}
 style={{
 display: "flex",
 alignItems: "flex-start",
 gap: 8,
 padding: "8px 10px",
 borderRadius: "var(--radius-sm)",
 background: f.pinned ? "rgba(137,180,250,0.08)" : "var(--bg-tertiary)",
 border: f.pinned ? "1px solid rgba(137,180,250,0.3)" : "1px solid var(--border-color)",
 }}
 >
 <button
 onClick={() => togglePin(f.id, f.pinned)}
 title={f.pinned ? "Unpin" : "Pin"}
 style={{ background: "none", border: "none", cursor: "pointer", fontSize: "var(--font-size-lg)", opacity: f.pinned ? 1 : 0.4, flexShrink: 0, padding: 0, lineHeight: 1, color: "var(--info-color)" }}
 >
 <Pin size={14} strokeWidth={1.5} />
 </button>
 <div style={{ flex: 1, minWidth: 0 }}>
 <div style={{ fontSize: "var(--font-size-base)", lineHeight: 1.5 }}>{f.fact}</div>
 <div style={{ display: "flex", gap: 6, marginTop: 4, flexWrap: "wrap", alignItems: "center" }}>
 <span style={{ fontSize: "var(--font-size-xs)", color: confidenceColor(f.confidence), fontWeight: 600 }}>
 {Math.round(f.confidence * 100)}% conf
 </span>
 {f.tags.map((t) => (
 <span key={t} style={{ fontSize: "var(--font-size-xs)", padding: "1px 5px", borderRadius: 3, background: "rgba(255,255,255,0.08)", color: "var(--text-secondary)" }}>
 {t}
 </span>
 ))}
 {f.session_id && (
 <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", opacity: 0.6 }}>
 from session {f.session_id.slice(0, 8)}
 </span>
 )}
 </div>
 </div>
 <button
 onClick={() => deleteFact(f.id)}
 title="Delete"
 style={{ background: "none", border: "none", cursor: "pointer", fontSize: "var(--font-size-base)", color: "var(--text-secondary)", flexShrink: 0, padding: "0 2px", lineHeight: 1 }}
 >
 ✕
 </button>
 </div>
 ))}
 </div>

 <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
 Stored at <code style={{ fontSize: "var(--font-size-xs)" }}>~/.vibeui/auto-memory.json</code>
 </div>
 </div>
 );
}

// -- MemoryPanel --------------------------------------------------------------

export function MemoryPanel({ workspacePath }: MemoryPanelProps) {
 const { toasts, toast, dismiss } = useToast();
 const [activeTab, setActiveTab] = useState<RulesTab>("workspace");
 const [workspaceRules, setWorkspaceRules] = useState("");
 const [globalRules, setGlobalRules] = useState("");
 const [saving, setSaving] = useState(false);
 const [saved, setSaved] = useState(false);
 const [generating, setGenerating] = useState(false);

 useEffect(() => {
 let cancelled = false;
 if (workspacePath) {
 invoke<string>("get_vibeui_rules")
 .then((r) => { if (!cancelled) setWorkspaceRules(r); })
 .catch(() => { if (!cancelled) setWorkspaceRules(""); });
 }
 invoke<string>("get_global_rules")
 .then((r) => { if (!cancelled) setGlobalRules(r); })
 .catch(() => { if (!cancelled) setGlobalRules(""); });
 return () => { cancelled = true; };
 }, [workspacePath]);

 const save = async () => {
 setSaving(true);
 setSaved(false);
 try {
 if (activeTab === "workspace") {
 await invoke("save_vibeui_rules", { content: workspaceRules });
 } else {
 await invoke("save_global_rules", { content: globalRules });
 }
 setSaved(true);
 setTimeout(() => setSaved(false), 2000);
 } catch (e) {
 toast.error("Failed to save: " + e);
 } finally {
 setSaving(false);
 }
 };

 const generateRules = async () => {
  if (activeTab !== "workspace") return;
  setGenerating(true);
  try {
   const generated = await invoke<string>("generate_vibeui_rules");
   setWorkspaceRules(generated);
  } catch (e) {
   toast.error("Generate failed: " + e);
  } finally {
   setGenerating(false);
  }
 };

 const placeholder =
 activeTab === "workspace"
 ? `# Project AI Rules\n\nInstructions injected into every AI request for this project.\n\nExamples:\n- Always use TypeScript strict mode\n- Prefer async/await over .then()\n- Use PostgreSQL for database operations\n- Follow the existing folder structure in src/`
 : `# Global AI Rules\n\nPersonal defaults applied to all projects.\n\nExamples:\n- Always add error handling\n- Write tests for every new function\n- Use descriptive variable names\n- Prefer immutability`;

 const tabs: { id: RulesTab; label: string }[] = [
 { id: "workspace", label: "Project" },
 { id: "global", label: "Global" },
 { id: "directory", label: "Dir Rules" },
 { id: "auto", label: "Auto-Facts" },
 ];

 return (
 <div style={{ display: "flex", flexDirection: "column", flex: 1, minHeight: 0, padding: "12px", gap: "8px" }}>
 <div style={{ fontWeight: 600, fontSize: "var(--font-size-lg)" }}>AI Rules / Memory</div>
 <p style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", margin: 0 }}>
 Persistent instructions injected into every AI request.
 </p>

 {/* Tab selector */}
 <div style={{ display: "flex", gap: "4px" }}>
 {tabs.map((t) => (
 <button
 key={t.id}
 onClick={() => setActiveTab(t.id)}
 style={{
 padding: "4px 10px",
 fontSize: "var(--font-size-base)",
 borderRadius: "var(--radius-xs-plus)",
 background: activeTab === t.id ? "var(--accent-color)" : "var(--bg-tertiary)",
 color: activeTab === t.id ? "white" : "var(--text-primary)",
 border: "1px solid var(--border-color)",
 cursor: "pointer",
 }}
 >
 {t.label}
 </button>
 ))}
 </div>

 {/* Directory rules tab */}
 {activeTab === "directory" && (
 <div style={{ flex: 1, minHeight: 0, position: "relative" }}>
 <DirRulesTab workspacePath={workspacePath} />
 </div>
 )}

 {/* Auto-Facts tab */}
 {activeTab === "auto" && (
 <div style={{ flex: 1, minHeight: 0, overflow: "hidden" }}>
 <AutoFactsTab />
 </div>
 )}

 {/* Single-file rules tabs */}
 {activeTab !== "directory" && activeTab !== "auto" && (
 <>
 {activeTab === "workspace" && !workspacePath && (
 <div style={{ fontSize: "var(--font-size-base)", color: "var(--warning-color)", padding: "6px", background: "color-mix(in srgb, var(--accent-rose) 10%, transparent)", borderRadius: "var(--radius-xs-plus)" }}>
 Open a folder to manage project rules.
 </div>
 )}

 <textarea
 value={activeTab === "workspace" ? workspaceRules : globalRules}
 onChange={(e) =>
 activeTab === "workspace"
 ? setWorkspaceRules(e.target.value)
 : setGlobalRules(e.target.value)
 }
 placeholder={placeholder}
 disabled={activeTab === "workspace" && !workspacePath}
 style={{
 flex: 1,
 background: "var(--bg-tertiary)",
 border: "1px solid var(--border-color)",
 color: "var(--text-primary)",
 borderRadius: "var(--radius-xs-plus)",
 padding: "8px",
 fontSize: "var(--font-size-md)",
 fontFamily: "var(--font-mono)",
 resize: "none",
 opacity: activeTab === "workspace" && !workspacePath ? 0.5 : 1,
 }}
 />

 <div style={{ display: "flex", gap: "6px" }}>
  {activeTab === "workspace" && (
   <button
    className="panel-btn panel-btn-secondary"
    onClick={generateRules}
    disabled={generating || !workspacePath}
    title="Generate rules from project stack"
   >
    {generating ? "Generating…" : "Generate with AI"}
   </button>
  )}
  <button
   className="panel-btn panel-btn-primary"
   onClick={save}
   disabled={saving || (activeTab === "workspace" && !workspacePath)}
   style={{ marginLeft: "auto" }}
  >
   {saving ? "Saving…" : saved ? "✓ Saved!" : `Save ${activeTab === "workspace" ? "Project" : "Global"} Rules`}
  </button>
 </div>

 <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
 {activeTab === "workspace"
 ? "Saved to <workspace>/.vibeui.md — commit it with your project."
 : "Saved to ~/.vibeui/rules.md — applies to all projects."}
 </div>
 </>
 )}
 <Toaster toasts={toasts} onDismiss={dismiss} />
 </div>
 );
}
