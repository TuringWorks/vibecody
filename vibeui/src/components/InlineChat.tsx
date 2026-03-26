import { useEffect, useRef, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useVoiceInput } from "../hooks/useVoiceInput";

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
 /** Full file content for generate mode context */
 fileContent?: string;
 onAccept: (newText: string) => void;
 onReject: () => void;
}

export function InlineChat({ selection, position, provider, fileContent, onAccept, onReject }: InlineChatProps) {
 const [prompt, setPrompt] = useState("");
 const [response, setResponse] = useState("");
 const [loading, setLoading] = useState(false);
 const inputRef = useRef<HTMLTextAreaElement>(null);

 const isGenerateMode = !selection.text.trim();

 const { isListening, isTranscribing, toggle: toggleVoice } = useVoiceInput((transcript) => {
  setPrompt(prev => prev ? prev + " " + transcript : transcript);
 });

 useEffect(() => {
  inputRef.current?.focus();
 }, []);

 const handleSubmit = async () => {
  if (!prompt.trim()) return;
  setLoading(true);
  setResponse("");
  try {
   if (isGenerateMode) {
    // Generate mode: insert new code at cursor
    const result = await invoke<string>("generate_code", {
     filePath: selection.filePath,
     language: selection.language,
     fileContent: fileContent || "",
     cursorLine: selection.startLine,
     instruction: prompt,
     provider,
    });
    setResponse(result);
   } else {
    // Edit mode: replace selected text
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
   }
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

 // Dragging state
 const [pos, setPos] = useState(() => ({
  top: Math.max(8, Math.min(position.top, window.innerHeight - 320)),
  left: Math.max(8, Math.min(position.left, window.innerWidth - 420)),
 }));
 const dragging = useRef(false);
 const dragOffset = useRef({ x: 0, y: 0 });

 const onDragStart = useCallback((e: React.MouseEvent) => {
  dragging.current = true;
  dragOffset.current = { x: e.clientX - pos.left, y: e.clientY - pos.top };
  e.preventDefault();
 }, [pos]);

 useEffect(() => {
  const onMove = (e: MouseEvent) => {
   if (!dragging.current) return;
   setPos({
    top: Math.max(0, Math.min(e.clientY - dragOffset.current.y, window.innerHeight - 100)),
    left: Math.max(0, Math.min(e.clientX - dragOffset.current.x, window.innerWidth - 200)),
   });
  };
  const onUp = () => { dragging.current = false; };
  window.addEventListener("mousemove", onMove);
  window.addEventListener("mouseup", onUp);
  return () => { window.removeEventListener("mousemove", onMove); window.removeEventListener("mouseup", onUp); };
 }, []);

 return (
  <div
   className="inline-chat-overlay"
   style={{
    position: "fixed",
    top: pos.top,
    left: pos.left,
    width: 420,
    background: "var(--bg-secondary)",
    border: "1px solid var(--accent-color)",
    borderRadius: 8,
    boxShadow: "0 4px 24px rgba(0,0,0,0.5)",
    zIndex: 9999,
    padding: 12,
    display: "flex",
    flexDirection: "column",
    gap: 8,
   }}
  >
   {/* Header — draggable */}
   <div
    onMouseDown={onDragStart}
    style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 11, cursor: "grab", userSelect: "none" }}
   >
    <span style={{
     padding: "2px 6px", borderRadius: 3, fontWeight: 600, fontSize: 10,
     background: isGenerateMode ? "var(--accent-color)" : "var(--accent-bg)",
     color: isGenerateMode ? "var(--btn-primary-fg)" : "var(--accent-color)",
    }}>
     {isGenerateMode ? "GENERATE" : "EDIT"}
    </span>
    <span style={{ color: "var(--text-secondary)" }}>
     {isGenerateMode
      ? `Line ${selection.startLine + 1} · ${selection.language}`
      : `Lines ${selection.startLine + 1}–${selection.endLine + 1} · ${selection.language}`}
    </span>
    {selection.filePath && (
     <span style={{ color: "var(--text-secondary)", fontFamily: "var(--font-mono)", fontSize: 10, marginLeft: "auto" }}>
      {selection.filePath.split("/").pop()}
     </span>
    )}
   </div>

   {/* Selected code preview (edit mode only) */}
   {!isGenerateMode && (
    <pre
     style={{
      margin: 0, padding: "6px 8px", background: "var(--bg-tertiary)",
      borderRadius: 4, fontSize: 11, maxHeight: 80, overflow: "auto",
      color: "var(--text-primary)", whiteSpace: "pre-wrap", wordBreak: "break-all",
     }}
    >
     {selection.text.slice(0, 300)}{selection.text.length > 300 ? "…" : ""}
    </pre>
   )}

   {/* Instruction input + voice */}
   <div style={{ position: "relative" }}>
    <textarea
     ref={inputRef}
     value={prompt}
     onChange={(e) => setPrompt(e.target.value)}
     onKeyDown={handleKeyDown}
     placeholder={isListening ? "Listening…" : isGenerateMode
      ? "Describe what code to generate… (⌘↵ to submit)"
      : "Describe the edit… (⌘↵ to submit)"}
     style={{
      resize: "none", height: 60, width: "100%", boxSizing: "border-box",
      background: "var(--bg-primary)", border: "1px solid var(--border-color)",
      borderRadius: 4, color: "var(--text-primary)", fontSize: 13,
      padding: "6px 32px 6px 8px", outline: "none",
     }}
    />
    <button
     onClick={toggleVoice}
     disabled={isTranscribing}
     title={isListening ? "Stop listening" : "Voice input"}
     style={{
      position: "absolute", right: 4, top: 4,
      width: 24, height: 24, borderRadius: 4, border: "none",
      background: isListening ? "var(--error-color)" : "transparent",
      color: isListening ? "var(--btn-primary-fg)" : "var(--text-secondary)",
      cursor: isTranscribing ? "wait" : "pointer", display: "flex",
      alignItems: "center", justifyContent: "center", padding: 0,
      transition: "background 0.15s, color 0.15s",
     }}
    >
     <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/><line x1="12" x2="12" y1="19" y2="22"/>
     </svg>
    </button>
   </div>

   {/* Response area */}
   {(loading || response) && (
    <pre
     style={{
      margin: 0, padding: "6px 8px", background: "var(--bg-tertiary)",
      borderRadius: 4, fontSize: 12, maxHeight: 200, overflow: "auto",
      color: loading ? "var(--text-secondary)" : "var(--text-primary)",
      whiteSpace: "pre-wrap", wordBreak: "break-all",
     }}
    >
     {loading ? (isGenerateMode ? "Generating code…" : "Editing…") : response}
    </pre>
   )}

   {/* Action buttons */}
   <div style={{ display: "flex", gap: 6, justifyContent: "flex-end" }}>
    <button
     onClick={onReject}
     style={{
      padding: "4px 12px", fontSize: 12, background: "transparent",
      border: "1px solid var(--border-color)", borderRadius: 4,
      color: "var(--text-secondary)", cursor: "pointer",
     }}
    >
     Cancel
    </button>
    {!response && (
     <button
      onClick={handleSubmit}
      disabled={loading || !prompt.trim()}
      style={{
       padding: "4px 12px", fontSize: 12, background: "var(--accent-color)",
       border: "none", borderRadius: 4, color: "var(--btn-primary-fg)",
       cursor: loading ? "wait" : "pointer", opacity: !prompt.trim() ? 0.5 : 1,
      }}
     >
      {loading ? "…" : isGenerateMode ? "Generate ⌘↵" : "Edit ⌘↵"}
     </button>
    )}
    {response && !loading && (
     <>
      <button
       onClick={() => { setResponse(""); setPrompt(""); }}
       style={{
        padding: "4px 12px", fontSize: 12, background: "transparent",
        border: "1px solid var(--border-color)", borderRadius: 4,
        color: "var(--text-secondary)", cursor: "pointer",
       }}
      >
       Retry
      </button>
      <button
       onClick={() => onAccept(response)}
       style={{
        padding: "4px 12px", fontSize: 12, background: "var(--accent-color)",
        border: "none", borderRadius: 4, color: "var(--btn-primary-fg)", cursor: "pointer",
       }}
      >
       {isGenerateMode ? "Insert" : "Accept"}
      </button>
     </>
    )}
   </div>
  </div>
 );
}
