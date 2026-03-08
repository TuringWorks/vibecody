import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
// lucide-react icons not needed

export interface InlineChatSelection {
 text: string;
 startLine: number;
 endLine: number;
 filePath: string;
 language: string;
}

interface InlineChatProps {
 selection: InlineChatSelection;
 position: { top: number; left: number };
 provider: string;
 onAccept: (newText: string) => void;
 onReject: () => void;
}

export function InlineChat({ selection, position, provider, onAccept, onReject }: InlineChatProps) {
 const [prompt, setPrompt] = useState("");
 const [response, setResponse] = useState("");
 const [loading, setLoading] = useState(false);
 const inputRef = useRef<HTMLTextAreaElement>(null);

 useEffect(() => {
 inputRef.current?.focus();
 }, []);

 const handleSubmit = async () => {
 if (!prompt.trim()) return;
 setLoading(true);
 setResponse("");
 try {
 const result = await invoke<string>("inline_edit", {
 filePath: selection.filePath,
 language: selection.language,
 selectedText: selection.text,
 startLine: selection.startLine,
 endLine: selection.endLine,
 instruction: prompt,
 provider,
 });
 setResponse(result);
 } catch (e) {
 setResponse(`Error: ${e}`);
 } finally {
 setLoading(false);
 }
 };

 const handleKeyDown = (e: React.KeyboardEvent) => {
 if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
 e.preventDefault();
 handleSubmit();
 }
 if (e.key === "Escape") {
 e.preventDefault();
 onReject();
 }
 };

 // Clamp position so overlay doesn't go offscreen
 const safeTop = Math.min(position.top, window.innerHeight - 320);
 const safeLeft = Math.min(position.left, window.innerWidth - 420);

 return (
 <div
 className="inline-chat-overlay"
 style={{
 position: "fixed",
 top: Math.max(8, safeTop),
 left: Math.max(8, safeLeft),
 width: 400,
 background: "var(--bg-secondary, #1e1e1e)",
 border: "1px solid var(--accent-blue, #0078d4)",
 borderRadius: 6,
 boxShadow: "0 4px 20px rgba(0,0,0,0.5)",
 zIndex: 9999,
 padding: 12,
 display: "flex",
 flexDirection: "column",
 gap: 8,
 }}
 >
 <div style={{ fontSize: 11, color: "var(--text-secondary, #888)", marginBottom: 2 }}>
 Inline Edit &nbsp;
 <span style={{ opacity: 0.6 }}>
 Lines {selection.startLine + 1}–{selection.endLine + 1} · {selection.language}
 </span>
 </div>

 {/* Selected code preview */}
 <pre
 style={{
 margin: 0,
 padding: "6px 8px",
 background: "var(--bg-tertiary, #2d2d2d)",
 borderRadius: 4,
 fontSize: 11,
 maxHeight: 80,
 overflow: "auto",
 color: "var(--text-primary, #ccc)",
 whiteSpace: "pre-wrap",
 wordBreak: "break-all",
 }}
 >
 {selection.text.slice(0, 300)}{selection.text.length > 300 ? "…" : ""}
 </pre>

 {/* Instruction input */}
 <textarea
 ref={inputRef}
 value={prompt}
 onChange={(e) => setPrompt(e.target.value)}
 onKeyDown={handleKeyDown}
 placeholder="Describe the edit… (Ctrl/Cmd+Enter to submit, Esc to cancel)"
 style={{
 resize: "none",
 height: 60,
 background: "var(--bg-primary, #141414)",
 border: "1px solid var(--border-color, #333)",
 borderRadius: 4,
 color: "var(--text-primary, #eee)",
 fontSize: 13,
 padding: "6px 8px",
 outline: "none",
 }}
 />

 {/* Response area */}
 {(loading || response) && (
 <pre
 style={{
 margin: 0,
 padding: "6px 8px",
 background: "var(--bg-tertiary, #2d2d2d)",
 borderRadius: 4,
 fontSize: 12,
 maxHeight: 120,
 overflow: "auto",
 color: loading ? "var(--text-secondary, #888)" : "var(--text-primary, #ccc)",
 whiteSpace: "pre-wrap",
 wordBreak: "break-all",
 }}
 >
 {loading ? "Generating…" : response}
 </pre>
 )}

 {/* Action buttons */}
 <div style={{ display: "flex", gap: 6, justifyContent: "flex-end" }}>
 <button
 onClick={onReject}
 style={{
 padding: "4px 12px",
 fontSize: 12,
 background: "transparent",
 border: "1px solid var(--border-color, #444)",
 borderRadius: 4,
 color: "var(--text-secondary, #888)",
 cursor: "pointer",
 }}
 >
 Cancel
 </button>
 {!response && (
 <button
 onClick={handleSubmit}
 disabled={loading || !prompt.trim()}
 style={{
 padding: "4px 12px",
 fontSize: 12,
 background: "var(--accent-blue, #0078d4)",
 border: "none",
 borderRadius: 4,
 color: "var(--text-on-accent, #fff)",
 cursor: loading ? "wait" : "pointer",
 opacity: !prompt.trim() ? 0.5 : 1,
 }}
 >
 {loading ? "…" : "Generate ⌘↵"}
 </button>
 )}
 {response && !loading && (
 <button
 onClick={() => onAccept(response)}
 style={{
 padding: "4px 12px",
 fontSize: 12,
 background: "var(--accent-green, #14a83c)",
 border: "none",
 borderRadius: 4,
 color: "var(--text-on-accent, #fff)",
 cursor: "pointer",
 }}
 >
 Accept
 </button>
 )}
 </div>
 </div>
 );
}
