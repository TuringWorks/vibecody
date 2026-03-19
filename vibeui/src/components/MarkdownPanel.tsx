/**
 * MarkdownPanel — Markdown Editor & Previewer.
 *
 * Split-pane editor with live rendered preview (react-markdown).
 * Browse .md files from the workspace, create new notes, save, and
 * export to standalone HTML.
 */
import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import ReactMarkdown from "react-markdown";

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
 const mins = Math.max(1, Math.round(words / 200));
 return `${mins} min read`;
}

function toHtml(md: string, title: string): string {
 // Very lightweight export — wraps content in a styled page.
 // For full fidelity a proper md→html transform would be used,
 // but since we render with react-markdown we snapshot innerHTML via a ref.
 return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>${title}</title>
<style>
 body{max-width:720px;margin:40px auto;font-family:system-ui,sans-serif;line-height:1.7;color:#24292f}
 pre{background:#f6f8fa;padding:16px;border-radius:6px;overflow:auto}
 code{background:#f6f8fa;padding:2px 5px;border-radius:4px;font-size:.9em}
 blockquote{border-left:4px solid #d0d7de;margin:0;padding:0 16px;color:#57606a}
 img{max-width:100%}
 table{border-collapse:collapse;width:100%}
 th,td{border:1px solid #d0d7de;padding:8px 12px}
</style>
</head>
<body>
${md.replace(/\n/g, "\n")}
</body>
</html>`;
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
 } catch { /* no workspace */ }
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
 const path = filePath ?? (workspacePath ? `${workspacePath}/${fileName}` : null);
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
 const html = toHtml(content, fileName.replace(/\.mdx?$/, ""));
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

 return (
 <div style={{ display: "flex", height: "100%", overflow: "hidden" }}>
 {/* File sidebar */}
 <div style={{ width: 190, borderRight: "1px solid var(--border-color)", display: "flex", flexDirection: "column", flexShrink: 0 }}>
 <div style={{ padding: "8px 10px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", alignItems: "center", gap: 6 }}>
 <span style={{ fontSize: 11, fontWeight: 600, flex: 1 }}>Files</span>
 <button onClick={newFile} title="New file" style={{ fontSize: 13, background: "none", border: "none", color: "var(--accent-primary)", cursor: "pointer", fontWeight: 700, lineHeight: 1 }}>+</button>
 <button onClick={loadFiles} title="Refresh" style={{ fontSize: 11, background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer" }}>↺</button>
 </div>
 <div style={{ padding: "6px 8px", borderBottom: "1px solid var(--border-color)" }}>
 <input
 value={filter}
 onChange={e => setFilter(e.target.value)}
 placeholder="Filter files…"
 style={{ width: "100%", padding: "3px 7px", fontSize: 10, background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none", boxSizing: "border-box" }}
 />
 </div>
 <div style={{ flex: 1, overflowY: "auto" }}>
 {!workspacePath && (
 <div style={{ padding: 12, fontSize: 10, color: "var(--text-muted)", textAlign: "center" }}>Open a workspace folder to browse files</div>
 )}
 {filtered.map(f => (
 <button
 key={f.path}
 onClick={() => openFile(f)}
 style={{
 display: "block", width: "100%", textAlign: "left",
 padding: "7px 10px", cursor: "pointer", fontSize: 11,
 background: filePath === f.path ? "var(--accent-bg, rgba(99,102,241,0.15))" : "transparent",
 border: "none", borderBottom: "1px solid var(--border-color)",
 color: "var(--text-primary)",
 }}
 >
 <div style={{ fontWeight: filePath === f.path ? 600 : 400, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{f.name}</div>
 <div style={{ fontSize: 9, color: "var(--text-muted)" }}>{(f.size_bytes / 1024).toFixed(1)} KB</div>
 </button>
 ))}
 {workspacePath && filtered.length === 0 && (
 <div style={{ padding: 12, fontSize: 10, color: "var(--text-muted)", textAlign: "center" }}>No .md files found</div>
 )}
 </div>
 </div>

 {/* Editor area */}
 <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
 {/* Toolbar */}
 <div style={{ padding: "6px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
 <span style={{ fontSize: 12, fontWeight: 600, flex: 1, color: dirty ? "var(--warning-color)" : "var(--text-primary)" }}>
 {fileName}{dirty ? " •" : ""}
 </span>

 {/* View toggle */}
 {(["split", "editor", "preview"] as View[]).map(v => (
 <button key={v} onClick={() => setView(v)} style={{ padding: "2px 10px", fontSize: 10, borderRadius: 10, background: view === v ? "rgba(99,102,241,0.25)" : "var(--bg-primary)", border: `1px solid ${view === v ? "var(--accent-color)" : "var(--border-color)"}`, color: view === v ? "var(--accent-color)" : "var(--text-muted)", cursor: "pointer", fontWeight: view === v ? 700 : 400 }}>
 {v === "split" ? "Split" : v === "editor" ? "Edit" : "Preview"}
 </button>
 ))}

 <button onClick={save} disabled={saving} style={{ padding: "3px 12px", fontSize: 11, fontWeight: 700, background: "var(--accent-color)", border: "none", borderRadius: 4, color: "var(--text-primary)", cursor: saving ? "not-allowed" : "pointer" }}>
 {saving ? "" : "Save"}
 </button>
 <button onClick={exportHtml} style={{ padding: "3px 10px", fontSize: 11, background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-muted)", cursor: "pointer" }}>
 ↗ HTML
 </button>
 </div>

 {/* Status bar */}
 {status && (
 <div style={{ padding: "3px 12px", fontSize: 10, background: status.startsWith("Error") ? "rgba(243,139,168,0.1)" : "rgba(166,227,161,0.1)", color: status.startsWith("Error") ? "var(--error-color)" : "var(--success-color)", borderBottom: "1px solid var(--border-color)" }}>
 {status}
 </div>
 )}

 {/* Panes */}
 <div style={{ flex: 1, display: "flex", overflow: "hidden" }}>
 {/* Editor pane */}
 {view !== "preview" && (
 <div style={{ flex: 1, display: "flex", flexDirection: "column", borderRight: view === "split" ? "1px solid var(--border-color)" : "none" }}>
 <textarea
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
 const next = content.slice(0, start) + " " + content.slice(end);
 setContent(next);
 setDirty(true);
 requestAnimationFrame(() => { el.selectionStart = el.selectionEnd = start + 2; });
 }
 }}
 spellCheck={false}
 style={{
 flex: 1, resize: "none", padding: "14px 16px",
 fontSize: 13, fontFamily: "var(--font-mono)", lineHeight: 1.7,
 background: "var(--bg-primary)", color: "var(--text-primary)",
 border: "none", outline: "none",
 }}
 />
 {/* Stats footer */}
 <div style={{ padding: "3px 14px", borderTop: "1px solid var(--border-color)", background: "var(--bg-secondary)", fontSize: 9, color: "var(--text-muted)", display: "flex", gap: 12 }}>
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
 fontSize: 14, lineHeight: 1.8, color: "var(--text-primary)",
 }}>
 <ReactMarkdown
 components={{
 h1: ({ children }) => <h1 style={{ fontSize: 28, fontWeight: 700, borderBottom: "1px solid var(--border-color)", paddingBottom: 8, marginBottom: 16 }}>{children}</h1>,
 h2: ({ children }) => <h2 style={{ fontSize: 22, fontWeight: 600, marginTop: 28, marginBottom: 10 }}>{children}</h2>,
 h3: ({ children }) => <h3 style={{ fontSize: 18, fontWeight: 600, marginTop: 20, marginBottom: 8 }}>{children}</h3>,
 p: ({ children }) => <p style={{ margin: "0 0 14px" }}>{children}</p>,
 code: ({ className, children }) => {
 const isBlock = className?.startsWith("language-");
 return isBlock
 ? <code style={{ display: "block", background: "var(--bg-secondary)", padding: "14px 16px", borderRadius: 6, fontSize: 12, fontFamily: "var(--font-mono)", overflowX: "auto", margin: "12px 0", whiteSpace: "pre" }}>{children}</code>
 : <code style={{ background: "var(--bg-secondary)", padding: "1px 5px", borderRadius: 3, fontSize: "0.9em", fontFamily: "var(--font-mono)" }}>{children}</code>;
 },
 pre: ({ children }) => <>{children}</>,
 blockquote: ({ children }) => <blockquote style={{ borderLeft: "3px solid var(--accent-color)", margin: "16px 0", paddingLeft: 16, color: "var(--text-muted)", fontStyle: "italic" }}>{children}</blockquote>,
 ul: ({ children }) => <ul style={{ paddingLeft: 24, margin: "10px 0" }}>{children}</ul>,
 ol: ({ children }) => <ol style={{ paddingLeft: 24, margin: "10px 0" }}>{children}</ol>,
 li: ({ children }) => <li style={{ marginBottom: 4 }}>{children}</li>,
 a: ({ href, children }) => <a href={href} target="_blank" rel="noreferrer" style={{ color: "var(--text-info)" }}>{children}</a>,
 hr: () => <hr style={{ border: "none", borderTop: "1px solid var(--border-color)", margin: "24px 0" }} />,
 table: ({ children }) => <table style={{ borderCollapse: "collapse", width: "100%", margin: "16px 0" }}>{children}</table>,
 th: ({ children }) => <th style={{ border: "1px solid var(--border-color)", padding: "6px 12px", background: "var(--bg-secondary)", fontWeight: 600 }}>{children}</th>,
 td: ({ children }) => <td style={{ border: "1px solid var(--border-color)", padding: "6px 12px" }}>{children}</td>,
 img: ({ src, alt }) => <img src={src} alt={alt ?? ""} style={{ maxWidth: "100%", borderRadius: 6 }} />,
 }}
 >
 {content}
 </ReactMarkdown>
 </div>
 </div>
 )}
 </div>
 </div>
 </div>
 );
}
