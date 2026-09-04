/**
 * MarkdownPanel — Markdown Editor & Previewer.
 *
 * Split-pane editor with live rendered preview (react-markdown).
 * Browse .md files from the workspace, create new notes, save, and
 * export to standalone HTML.
 */
import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { MarkdownWithDetails } from "./MarkdownDetails";
import { renderBlock, renderDocumentHtml, renderSummary } from "./markdownDocument";

interface MarkdownFile {
 path: string;
 name: string;
 size_bytes: number;
}

const DEFAULT_CONTENT = `# Untitled

Start writing **Markdown** here.

## Features

- Live preview
- File browser
- Export to HTML
- Word count & reading time

\`\`\`typescript
const hello = "world";
console.log(hello);
\`\`\`

>Tip: Use the sidebar to open existing \`.md\` files from your workspace.
`;

function wordCount(text: string): number {
 return text.trim().split(/\s+/).filter(Boolean).length;
}

function readingTime(words: number): string {
 const mins = Math.max(1, Math.ceil(words / 200));
 return `${mins} min read`;
}


type View = "split" | "editor" | "preview";

export function MarkdownPanel({ workspacePath }: { workspacePath: string | null }) {
 const [files, setFiles] = useState<MarkdownFile[]>([]);
 const [content, setContent] = useState(DEFAULT_CONTENT);
 const [filePath, setFilePath] = useState<string | null>(null);
 const [fileName, setFileName] = useState("untitled.md");
 const [dirty, setDirty] = useState(false);
 const [view, setView] = useState<View>("split");
 const [filter, setFilter] = useState("");
 const [saving, setSaving] = useState(false);
 const [status, setStatus] = useState<string | null>(null);
 const previewRef = useRef<HTMLDivElement>(null);

 // Load file list
 const loadFiles = useCallback(async () => {
 if (!workspacePath) return;
 try {
 const list = await invoke<MarkdownFile[]>("list_markdown_files", { workspace: workspacePath });
 setFiles(list);
 } catch (e) {
 setStatus(`Load failed: ${e}`);
 }
 }, [workspacePath]);

 useEffect(() => { loadFiles(); }, [loadFiles]);

 const openFile = async (f: MarkdownFile) => {
 try {
 const data = await invoke<string>("read_file", { path: f.path });
 setContent(data);
 setFilePath(f.path);
 setFileName(f.name);
 setDirty(false);
 } catch (e) {
 setStatus(`Error: ${e}`);
 }
 };

 const newFile = () => {
 setContent(DEFAULT_CONTENT);
 setFilePath(null);
 setFileName("untitled.md");
 setDirty(false);
 };

 const save = async () => {
 const trimmedName = fileName.trim();
 if (!filePath && (!trimmedName || trimmedName.includes("/") || trimmedName.includes("\\"))) {
 setStatus("Invalid filename — use a file name without folders");
 return;
 }
 const path = filePath ?? (workspacePath ? `${workspacePath}/${trimmedName}` : null);
 if (!path) { setStatus("No workspace — cannot save"); return; }
 setSaving(true);
 try {
 await invoke("write_file", { path, content });
 setFilePath(path);
 setDirty(false);
 setStatus("Saved ✓");
 setTimeout(() => setStatus(null), 2000);
 await loadFiles();
 } catch (e) {
 setStatus(`Save failed: ${e}`);
 } finally {
 setSaving(false);
 }
 };

 const exportHtml = () => {
 const html = renderDocumentHtml(content, fileName.replace(/\.mdx?$/, ""));
 const blob = new Blob([html], { type: "text/html" });
 const url = URL.createObjectURL(blob);
 const a = document.createElement("a");
 a.href = url;
 a.download = fileName.replace(/\.mdx?$/, ".html");
 a.click();
 URL.revokeObjectURL(url);
 };

 const words = wordCount(content);
 const chars = content.length;
 const filtered = files.filter(f => !filter || f.name.toLowerCase().includes(filter.toLowerCase()));
 const statusIsError = status?.startsWith("Error") || status?.startsWith("Save failed") || status?.startsWith("Load failed") || status?.startsWith("No workspace") || status?.startsWith("Invalid filename");

 return (
 <div className="panel-container" style={{ display: "flex", flex: 1, minHeight: 0, overflow: "hidden" }}>
 {/* File sidebar */}
 <div style={{ width: 190, borderRight: "1px solid var(--border-color)", display: "flex", flexDirection: "column", flexShrink: 0 }}>
 <div style={{ padding: "8px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", alignItems: "center", gap: 6 }}>
 <span style={{ fontSize: "var(--font-size-sm)", fontWeight: 600, flex: 1 }}>Files</span>
 <button aria-label="New file" className="panel-btn" onClick={newFile} title="New file" style={{ fontSize: "var(--font-size-md)", background: "none", border: "none", color: "var(--accent-primary)", cursor: "pointer", fontWeight: 700, lineHeight: 1 }}>+</button>
 <button aria-label="Refresh Markdown files" className="panel-btn" onClick={loadFiles} title="Refresh" style={{ fontSize: "var(--font-size-sm)", background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer" }}>↺</button>
 </div>
 <div style={{ padding: "8px 8px", borderBottom: "1px solid var(--border-color)" }}>
 <input
 aria-label="Filter Markdown files"
 value={filter}
 onChange={e => setFilter(e.target.value)}
 placeholder="Filter files…"
 style={{ width: "100%", padding: "3px 8px", fontSize: "var(--font-size-xs)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", outline: "none", boxSizing: "border-box" }}
 />
 </div>
 <div style={{ flex: 1, overflowY: "auto" }}>
 {!workspacePath && (
 <div style={{ padding: 12, fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", textAlign: "center" }}>Open a workspace folder to browse files</div>
 )}
 {filtered.map(f => (
 <button
 key={f.path}
 onClick={() => openFile(f)}
 style={{
 display: "block", width: "100%", textAlign: "left",
 padding: "8px 12px", cursor: "pointer", fontSize: "var(--font-size-sm)",
 background: filePath === f.path ? "var(--accent-bg, color-mix(in srgb, var(--accent-blue) 15%, transparent))" : "transparent",
 border: "none", borderBottom: "1px solid var(--border-color)",
 color: "var(--text-primary)",
 }}
 >
 <div style={{ fontWeight: filePath === f.path ? 600 : 400, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{f.name}</div>
 <div style={{ fontSize: 9, color: "var(--text-secondary)" }}>{(f.size_bytes / 1024).toFixed(1)} KB</div>
 </button>
 ))}
 {workspacePath && filtered.length === 0 && (
 <div style={{ padding: 12, fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", textAlign: "center" }}>No .md files found</div>
 )}
 </div>
 </div>

 {/* Editor area */}
 <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
 {/* Toolbar */}
 <div style={{ padding: "8px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
 {filePath
 ? <span style={{ fontSize: "var(--font-size-base)", fontWeight: 600, flex: 1, color: dirty ? "var(--warning-color)" : "var(--text-primary)" }}>{fileName}{dirty ? " •" : ""}</span>
 : <input aria-label="Markdown filename" value={fileName} onChange={e => { setFileName(e.target.value); setDirty(true); }} style={{ fontSize: "var(--font-size-base)", fontWeight: 600, flex: 1, minWidth: 100, color: dirty ? "var(--warning-color)" : "var(--text-primary)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", padding: "2px 6px" }} />}

 {/* View toggle */}
 {(["split", "editor", "preview"] as View[]).map(v => (
 <button key={v} onClick={() => setView(v)} style={{ padding: "2px 12px", fontSize: "var(--font-size-xs)", borderRadius: "var(--radius-md)", background: view === v ? "color-mix(in srgb, var(--accent-blue) 25%, transparent)" : "var(--bg-primary)", border: `1px solid ${view === v ? "var(--accent-color)" : "var(--border-color)"}`, color: view === v ? "var(--accent-color)" : "var(--text-secondary)", cursor: "pointer", fontWeight: view === v ? 700 : 400 }}>
 {v === "split" ? "Split" : v === "editor" ? "Edit" : "Preview"}
 </button>
 ))}

 <button className="panel-btn" onClick={save} disabled={saving} style={{ padding: "3px 12px", fontSize: "var(--font-size-sm)", fontWeight: 700, background: "var(--accent-color)", border: "none", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", cursor: saving ? "not-allowed" : "pointer" }}>
 {saving ? "Saving…" : "Save"}
 </button>
 <button aria-label="Export HTML" className="panel-btn" onClick={exportHtml} style={{ padding: "3px 12px", fontSize: "var(--font-size-sm)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-secondary)", cursor: "pointer" }}>
 ↗ HTML
 </button>
 </div>

 {/* Status bar */}
 {status && (
 <div role={statusIsError ? "alert" : "status"} style={{ padding: "3px 12px", fontSize: "var(--font-size-xs)", background: statusIsError ? "color-mix(in srgb, var(--accent-rose) 10%, transparent)" : "color-mix(in srgb, var(--accent-green) 10%, transparent)", color: statusIsError ? "var(--error-color)" : "var(--success-color)", borderBottom: "1px solid var(--border-color)" }}>
 {status}
 </div>
 )}

 {/* Panes */}
 <div style={{ flex: 1, display: "flex", overflow: "hidden" }}>
 {/* Editor pane */}
 {view !== "preview" && (
 <div style={{ flex: 1, display: "flex", flexDirection: "column", borderRight: view === "split" ? "1px solid var(--border-color)" : "none" }}>
 <textarea
 aria-label="Markdown editor"
 value={content}
 onChange={e => { setContent(e.target.value); setDirty(true); }}
 onKeyDown={e => {
 if ((e.ctrlKey || e.metaKey) && e.key === "s") { e.preventDefault(); save(); }
 // Tab → 2 spaces
 if (e.key === "Tab") {
 e.preventDefault();
 const el = e.target as HTMLTextAreaElement;
 const start = el.selectionStart;
 const end = el.selectionEnd;
 const next = content.slice(0, start) + "  " + content.slice(end);
 setContent(next);
 setDirty(true);
 requestAnimationFrame(() => { el.selectionStart = el.selectionEnd = start + 2; });
 }
 }}
 spellCheck={false}
 style={{
 flex: 1, resize: "none", padding: "16px 16px",
 fontSize: "var(--font-size-md)", fontFamily: "var(--font-mono)", lineHeight: 1.7,
 background: "var(--bg-primary)", color: "var(--text-primary)",
 border: "none", outline: "none",
 }}
 />
 {/* Stats footer */}
 <div style={{ padding: "3px 16px", borderTop: "1px solid var(--border-color)", background: "var(--bg-secondary)", fontSize: 9, color: "var(--text-secondary)", display: "flex", gap: 12 }}>
 <span>{words} words</span>
 <span>{chars} chars</span>
 <span>{readingTime(words)}</span>
 <span>{content.split("\n").length} lines</span>
 </div>
 </div>
 )}

 {/* Preview pane */}
 {view !== "editor" && (
 <div ref={previewRef} style={{ flex: 1, overflowY: "auto", padding: "20px 28px" }}>
 <div style={{
 maxWidth: 720,
 fontSize: "var(--font-size-lg)", lineHeight: 1.8, color: "var(--text-primary)",
 }}>
 <MarkdownWithDetails source={content} renderBlock={renderBlock} renderInline={renderSummary} />
 </div>
 </div>
 )}
 </div>
 </div>
 </div>
 );
}
