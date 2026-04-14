/**
 * ScaffoldPanel — Project Templates & Scaffolding.
 *
 * Browse built-in templates, preview generated files, pick an output directory,
 * and write the scaffold to disk in one click.
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ScaffoldTemplate {
 id: string;
 name: string;
 description: string;
 language: string;
 framework: string;
 tags: string[];
}

interface ScaffoldFile {
 path: string;
 content: string;
}

interface ScaffoldResult {
 files: ScaffoldFile[];
 install_command: string | null;
 dev_command: string | null;
 notes: string;
}

const LANG_COLORS: Record<string, string> = {
 Rust: "#f7a41d",
 TypeScript: "#3178c6",
 Python: "#3572A5",
 Go: "#00acd7",
 "Rust/TypeScript": "#9c7ce1",
};

function LangBadge({ lang }: { lang: string }) {
 const color = LANG_COLORS[lang] ?? "var(--text-secondary)";
 return (
 <span style={{ padding: "1px 7px", borderRadius: "var(--radius-md)", background: color + "33", border: `1px solid ${color}`, color, fontSize: "var(--font-size-xs)", fontWeight: 600 }}>
 {lang}
 </span>
 );
}

export function ScaffoldPanel({ workspacePath }: { workspacePath: string | null }) {
 const [templates, setTemplates] = useState<ScaffoldTemplate[]>([]);
 const [selected, setSelected] = useState<ScaffoldTemplate | null>(null);
 const [projectName, setProjectName] = useState("my-project");
 const [outputDir, setOutputDir] = useState("");
 const [filter, setFilter] = useState("");
 const [preview, setPreview] = useState<ScaffoldResult | null>(null);
 const [previewFile, setPreviewFile] = useState<ScaffoldFile | null>(null);
 const [generating, setGenerating] = useState(false);
 const [written, setWritten] = useState(false);
 const [error, setError] = useState<string | null>(null);

 useEffect(() => {
 invoke<ScaffoldTemplate[]>("list_scaffold_templates").then(setTemplates).catch(() => {});
 if (workspacePath) setOutputDir(workspacePath);
 }, [workspacePath]);

 const handleSelect = (t: ScaffoldTemplate) => {
 setSelected(t);
 setPreview(null);
 setPreviewFile(null);
 setWritten(false);
 setError(null);
 };

 const handlePreview = async () => {
 if (!selected) return;
 setGenerating(true);
 setError(null);
 try {
 const res = await invoke<ScaffoldResult>("generate_scaffold", {
 templateId: selected.id,
 projectName,
 outputDir: "", // dry run — no write
 });
 setPreview(res);
 setPreviewFile(res.files[0] ?? null);
 setWritten(false);
 } catch (e) {
 setError(String(e));
 } finally {
 setGenerating(false);
 }
 };

 const handleWrite = async () => {
 if (!selected || !outputDir.trim()) return;
 setGenerating(true);
 setError(null);
 try {
 await invoke<ScaffoldResult>("generate_scaffold", {
 templateId: selected.id,
 projectName,
 outputDir: outputDir.trim(),
 });
 setWritten(true);
 } catch (e) {
 setError(String(e));
 } finally {
 setGenerating(false);
 }
 };

 const filtered = templates.filter(t =>
 !filter || t.name.toLowerCase().includes(filter.toLowerCase()) ||
 t.language.toLowerCase().includes(filter.toLowerCase()) ||
 t.framework.toLowerCase().includes(filter.toLowerCase()) ||
 t.tags.some(tag => tag.includes(filter.toLowerCase()))
 );

 return (
 <div className="panel-container" style={{ flexDirection: "row" }}>
 {/* Template list */}
 <div style={{ width: 220, borderRight: "1px solid var(--border-color)", display: "flex", flexDirection: "column", flexShrink: 0 }}>
 <div style={{ padding: "10px 10px 6px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)" }}>
 <div style={{ fontSize: "var(--font-size-base)", fontWeight: 600, marginBottom: 6 }}>Scaffold</div>
 <input
 value={filter}
 onChange={e => setFilter(e.target.value)}
 placeholder="Filter templates…"
 style={{ width: "100%", padding: "4px 8px", fontSize: "var(--font-size-sm)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", outline: "none", boxSizing: "border-box" }}
 />
 </div>
 <div style={{ flex: 1, overflowY: "auto" }}>
 {filtered.map(t => (
 <button
 key={t.id}
 onClick={() => handleSelect(t)}
 style={{
 display: "block", width: "100%", textAlign: "left",
 padding: "8px 10px", cursor: "pointer",
 background: selected?.id === t.id ? "var(--accent-bg)" : "transparent",
 border: "none", borderBottom: "1px solid var(--border-color)",
 color: "var(--text-primary)",
 }}
 >
 <div style={{ fontSize: "var(--font-size-base)", fontWeight: 600 }}>{t.name}</div>
 <LangBadge lang={t.language} />
 <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", marginTop: 3, lineHeight: 1.3 }}>{t.description}</div>
 </button>
 ))}
 </div>
 </div>

 {/* Right pane */}
 <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
 {!selected ? (
 <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center", color: "var(--text-secondary)", fontSize: "var(--font-size-md)" }}>
 ← Select a template to get started
 </div>
 ) : (
 <>
 {/* Config bar */}
 <div style={{ padding: "10px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 8, alignItems: "flex-end", flexWrap: "wrap" }}>
 <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
 <label style={{ fontSize: "var(--font-size-xs)", fontWeight: 600, color: "var(--text-secondary)" }}>Project Name</label>
 <input
 value={projectName}
 onChange={e => setProjectName(e.target.value.replace(/[^a-zA-Z0-9_-]/g, ""))}
 style={{ padding: "4px 8px", fontSize: "var(--font-size-base)", fontFamily: "var(--font-mono)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", outline: "none", width: 160 }}
 />
 </div>
 <div style={{ display: "flex", flexDirection: "column", gap: 3, flex: 1 }}>
 <label style={{ fontSize: "var(--font-size-xs)", fontWeight: 600, color: "var(--text-secondary)" }}>Output Directory</label>
 <input
 value={outputDir}
 onChange={e => setOutputDir(e.target.value)}
 placeholder="/path/to/output"
 style={{ padding: "4px 8px", fontSize: "var(--font-size-sm)", fontFamily: "var(--font-mono)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", outline: "none" }}
 />
 </div>
 <button
 onClick={handlePreview}
 disabled={generating || !projectName}
 style={{ padding: "5px 14px", fontSize: "var(--font-size-sm)", fontWeight: 600, background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-secondary)", cursor: "pointer", height: 28 }}
 >
 Preview
 </button>
 <button
 onClick={handleWrite}
 disabled={generating || !projectName || !outputDir.trim()}
 style={{ padding: "5px 14px", fontSize: "var(--font-size-sm)", fontWeight: 700, background: generating ? "var(--bg-secondary)" : "var(--accent-color)", border: "none", borderRadius: "var(--radius-xs-plus)", color: generating ? "var(--text-secondary)" : "var(--text-primary)", cursor: generating || !outputDir.trim() ? "not-allowed" : "pointer", height: 28 }}
 >
 {generating ? "Writing…" : written ? "Written" : "Write Files"}
 </button>
 </div>

 {error && (
 <div style={{ padding: "6px 12px", background: "color-mix(in srgb, var(--accent-rose) 10%, transparent)", color: "var(--text-danger)", fontSize: "var(--font-size-sm)", borderBottom: "1px solid var(--border-color)" }}> {error}</div>
 )}

 {written && (
 <div style={{ padding: "6px 12px", background: "color-mix(in srgb, var(--accent-green) 10%, transparent)", color: "var(--text-success)", fontSize: "var(--font-size-sm)", borderBottom: "1px solid var(--border-color)" }}>
 Scaffold written to <code style={{ fontFamily: "var(--font-mono)" }}>{outputDir}</code>
 {preview?.install_command && <> — run <code style={{ fontFamily: "var(--font-mono)" }}>{preview.install_command}</code></>}
 </div>
 )}

 {/* File list + preview */}
 {preview ? (
 <div style={{ flex: 1, display: "flex", overflow: "hidden" }}>
 {/* File list */}
 <div style={{ width: 180, borderRight: "1px solid var(--border-color)", overflowY: "auto", flexShrink: 0 }}>
 {preview.files.map(f => (
 <button
 key={f.path}
 onClick={() => setPreviewFile(f)}
 style={{
 display: "block", width: "100%", textAlign: "left",
 padding: "6px 10px", fontSize: "var(--font-size-sm)", fontFamily: "var(--font-mono)",
 background: previewFile?.path === f.path ? "var(--accent-bg)" : "transparent",
 border: "none", borderBottom: "1px solid var(--border-color)",
 color: "var(--text-primary)", cursor: "pointer",
 wordBreak: "break-all",
 }}
 >
 {f.path}
 </button>
 ))}
 {/* Commands */}
 <div style={{ padding: "10px 10px", fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", borderTop: "1px solid var(--border-color)" }}>
 {preview.install_command && <div><b>Install:</b> {preview.install_command}</div>}
 {preview.dev_command && <div style={{ marginTop: 4 }}><b>Dev:</b> {preview.dev_command}</div>}
 {preview.notes && <div style={{ marginTop: 6, lineHeight: 1.4 }}>{preview.notes}</div>}
 </div>
 </div>

 {/* File content */}
 <div style={{ flex: 1, overflow: "auto" }}>
 {previewFile ? (
 <>
 <div style={{ padding: "6px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", fontSize: "var(--font-size-sm)", fontFamily: "var(--font-mono)", color: "var(--text-secondary)" }}>
 {previewFile.path}
 </div>
 <pre style={{ margin: 0, padding: "12px 14px", fontSize: "var(--font-size-sm)", fontFamily: "var(--font-mono)", lineHeight: 1.6, color: "var(--text-primary)", whiteSpace: "pre-wrap", wordBreak: "break-word" }}>
 {previewFile.content}
 </pre>
 </>
 ) : (
 <div style={{ padding: 20, color: "var(--text-secondary)", fontSize: "var(--font-size-base)" }}>Select a file to preview</div>
 )}
 </div>
 </div>
 ) : (
 <div style={{ flex: 1, display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: 8, color: "var(--text-secondary)" }}>
 <div style={{ fontSize: 32 }}></div>
 <div style={{ fontSize: "var(--font-size-md)", fontWeight: 600 }}>{selected.name}</div>
 <div style={{ fontSize: "var(--font-size-sm)" }}>{selected.description}</div>
 <div style={{ marginTop: 4, display: "flex", gap: 4 }}>
 {selected.tags.map(tag => (
 <span key={tag} style={{ padding: "2px 8px", borderRadius: "var(--radius-md)", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>{tag}</span>
 ))}
 </div>
 <div style={{ fontSize: "var(--font-size-sm)", marginTop: 8 }}>Click <b>Preview</b> to see generated files</div>
 </div>
 )}
 </>
 )}
 </div>
 </div>
 );
}
